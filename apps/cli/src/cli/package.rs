use anyhow::{Context, Result, anyhow};
use std::process::Command;
use clap::Args;
use fastforge_app_builder::{FlutterAppBuilder, Platform};
use fastforge_app_packager::{
    AppPackager, MacOSDmgPackager, MacOSPkgPackager, MacOSZipPackager, PackageConfig,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use serde_yaml;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Args)]
pub struct PackageArgs {
    #[arg(short, long = "platform")]
    pub platform: Option<String>,
    #[arg(short, long = "target")]
    pub target: Option<String>,
    #[arg(long = "skip-clean", default_value_t = false)]
    pub skip_clean: bool,
    #[arg(long = "build-target")]
    pub build_target: Option<String>,

    /// Shell command to run before packaging.
    #[arg(long = "hook-pre")]
    pub hook_pre: Option<String>,

    /// Shell command to run after packaging.
    #[arg(long = "hook-post")]
    pub hook_post: Option<String>,
}

pub async fn execute(args: &PackageArgs) -> Result<()> {
    log::info!("Executing package command");
    let platform = args
        .platform
        .as_deref()
        .ok_or_else(|| anyhow!("The 'platform' option is mandatory!"))?;
    let target = args
        .target
        .as_deref()
        .ok_or_else(|| anyhow!("The 'target' option is mandatory!"))?;

    let mut build_args = Map::new();
    if let Some(value) = &args.build_target {
        build_args.insert("target".to_string(), Value::String(value.clone()));
    }

    // Build hooks map from CLI args
    let hooks: Option<HashMap<String, serde_yaml::Value>> = {
        let mut map = HashMap::new();
        if let Some(cmd) = &args.hook_pre {
            map.insert("pre".to_string(), serde_yaml::Value::String(cmd.clone()));
        }
        if let Some(cmd) = &args.hook_post {
            map.insert("post".to_string(), serde_yaml::Value::String(cmd.clone()));
        }
        if map.is_empty() { None } else { Some(map) }
    };

    let artifacts = package_flutter_artifact(
        platform,
        target,
        build_args,
        std::env::vars().collect(),
        "dist/",
        None,
        !args.skip_clean,
        hooks.as_ref(),
    )?;

    for artifact in artifacts {
        println!("{}", artifact.display());
    }
    Ok(())
}

pub fn package_flutter_artifact(
    platform_str: &str,
    target: &str,
    build_args: Map<String, Value>,
    environment: HashMap<String, String>,
    output: &str,
    artifact_name: Option<String>,
    clean_before_build: bool,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Vec<PathBuf>> {
    let platform = Platform::from_str(platform_str)
        .map_err(|e| anyhow!("Invalid platform '{}': {}", platform_str, e))?;

    let builder = FlutterAppBuilder::default();
    if clean_before_build {
        builder
            .clean(Some(&environment))
            .map_err(|e| anyhow!("{}", e))?;
    }

    // Clone environment before it's moved into build()
    let hook_env_base = environment.clone();

    let build = builder
        .build(&platform, Some(target), build_args, Some(environment))
        .map_err(|e| anyhow!("{}", e))?;

    let pubspec = ProjectPubspec::load()?;
    let package_config = PackageConfig {
        app_name: pubspec.name.clone(),
        app_binary_name: pubspec.name,
        app_version: pubspec.version,
        build_mode: build.config.mode().as_str().to_string(),
        platform,
        flavor: build.config.flavor().map(ToOwned::to_owned),
        channel: None,
        artifact_name,
        package_format: target.to_string(),
        is_installer: matches!(
            target,
            "pkg" | "dmg" | "deb" | "rpm" | "pacman" | "exe" | "msix"
        ),
        build_output_dir: build.output_directory,
        build_output_files: build.output_files,
        output_dir: PathBuf::from(output),
    };

    let packager = macos_packager(target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }

    // Resolve hooks: YAML allows both a single string and a list of strings
    let pre_hooks = resolve_hooks(hooks, "pre");
    let post_hooks = resolve_hooks(hooks, "post");

    // Build hook environment
    let mut hook_env = hook_env_base;
    hook_env.insert(
        "PLATFORM".to_string(),
        package_config.platform.as_str().to_string(),
    );
    hook_env.insert(
        "PACKAGE_FORMAT".to_string(),
        package_config.package_format.clone(),
    );
    hook_env.insert(
        "BUILD_MODE".to_string(),
        package_config.build_mode.clone(),
    );
    hook_env.insert(
        "OUTPUT_DIRECTORY".to_string(),
        package_config.output_dir.to_string_lossy().to_string(),
    );
    hook_env.insert(
        "BUILD_OUTPUT_DIRECTORY".to_string(),
        package_config
            .build_output_dir
            .to_string_lossy()
            .to_string(),
    );
    hook_env.insert(
        "BUILD_OUTPUT_FILES".to_string(),
        package_config
            .build_output_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(":"),
    );

    // Run prepackage hooks
    run_hooks(&pre_hooks, &hook_env)?;

    let result = packager
        .package(&package_config)
        .map_err(|e| anyhow!("{}", e))?;

    // Run postpackage hooks
    run_hooks(&post_hooks, &hook_env)?;

    Ok(result.artifacts)
}

/// Extract and normalize hook commands for a given key ("pre" or "post").
/// Supports both a single string and a list of strings.
fn resolve_hooks(
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
    key: &str,
) -> Vec<String> {
    let Some(hooks) = hooks else { return vec![] };
    let Some(value) = hooks.get(key) else { return vec![] };
    match value {
        serde_yaml::Value::String(cmd) => vec![cmd.clone()],
        serde_yaml::Value::Sequence(seq) => {
            seq.iter().filter_map(|v| v.as_str().map(String::from)).collect()
        }
        _ => vec![],
    }
}

/// Execute a list of shell hook commands.
fn run_hooks(hooks: &[String], env: &HashMap<String, String>) -> Result<()> {
    for hook in hooks {
        let output = Command::new("sh")
            .args(["-c", hook])
            .envs(env)
            .output()
            .map_err(|e| anyhow!("Failed to execute hook '{}': {}", hook, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Hook failed (exit {}): {}\n{}",
                output.status.code().unwrap_or(-1),
                hook,
                stderr,
            ));
        }

        // Print hook stdout so users can see the output
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            print!("{}", stdout);
        }
    }
    Ok(())
}

fn macos_packager(target: &str) -> Result<Box<dyn AppPackager + Send + Sync>> {
    match target {
        "pkg" => Ok(Box::new(MacOSPkgPackager::from_yaml_file(
            std::path::Path::new("macos/packaging/pkg/make_config.yaml"),
        )?)),
        "dmg" => Ok(Box::new(MacOSDmgPackager)),
        "zip" => Ok(Box::new(MacOSZipPackager)),
        other => Err(anyhow!(
            "Unsupported package target: `{}`. Currently supported for CLI packaging: pkg, dmg, zip",
            other
        )),
    }
}

#[derive(Debug, Deserialize)]
struct ProjectPubspec {
    name: String,
    #[serde(default = "default_version")]
    version: String,
}

impl ProjectPubspec {
    fn load() -> Result<Self> {
        let content =
            std::fs::read_to_string("pubspec.yaml").context("Failed to read pubspec.yaml")?;
        serde_yaml::from_str(&content).context("Failed to parse pubspec.yaml")
    }
}

fn default_version() -> String {
    "0.1.0+1".to_string()
}
