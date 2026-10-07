use anyhow::{Context, Result, anyhow};
use clap::Args;
use fastforge_app_builder::{
    BuildError, BuildResult, FlutterAppBuilder, GradleAppBuilder, IOSXcodeAppBuilder,
    MacOSXcodeAppBuilder, Platform,
};
use fastforge_app_packager::{
    AndroidAabPackager, AndroidApkPackager, AppPackager, CustomPackager, IOSIpaPackager,
    LinuxAppImagePackager, LinuxDebPackager, LinuxDirectPackager, LinuxPacmanPackager,
    LinuxRpmPackager, LinuxZipPackager, MacOSDmgPackager, MacOSPkgPackager, MacOSZipPackager,
    OHOSAppPackager, OHOSHapPackager, PackageConfig, WebDirectPackager, WebZipPackager,
    WindowsDirectPackager, WindowsExePackager, WindowsMsixPackager, WindowsZipPackager,
};
use fastforge_core::{ProjectSettings, environment_variables};
use serde::Deserialize;
use serde_json::{Map, Value};
use serde_yaml;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use crate::config::DistributeOptions;
use crate::utils::{bright_green, global_variables, run_streaming, yellow};

#[derive(Args)]
pub struct PackageArgs {
    /// The platform to package the application for (auto-detected from the
    /// targets and project layout when omitted).
    #[arg(
        short,
        long = "platform",
        value_name = "android,ios,linux,macos,ohos,windows,web"
    )]
    pub platform: Option<String>,
    /// Comma separated list of bundle types to build.
    #[arg(
        short,
        long = "targets",
        alias = "target",
        value_name = "apk,aab,app,appimage,deb,dmg,exe,hap,ipa,msix,pkg,rpm,zip"
    )]
    pub targets: Option<String>,
    #[arg(long = "channel")]
    pub channel: Option<String>,
    /// Artifact name template (mustache syntax, e.g. `{{name}}-{{build_name}}.{{ext}}`).
    #[arg(long = "artifact-name")]
    pub artifact_name: Option<String>,
    /// Directory the packaged artifacts are written to. Defaults to the
    /// `output` key of `distribute_options.yaml`, or `dist/`.
    #[arg(long = "output")]
    pub output: Option<String>,
    /// Whether or not to skip 'flutter clean' before packaging.
    #[arg(long = "skip-clean", overrides_with = "no_skip_clean")]
    pub skip_clean: bool,
    #[arg(long = "no-skip-clean", overrides_with = "skip_clean", hide = true)]
    pub no_skip_clean: bool,

    /// Arguments to pass directly to flutter build
    #[arg(long = "flutter-build-args", value_name = "verbose,obfuscate")]
    pub flutter_build_args: Option<String>,
    /// The --target argument passed to 'flutter build'
    #[arg(long = "build-target", value_name = "path")]
    pub build_target: Option<String>,
    /// The --flavor argument passed to 'flutter build'
    #[arg(long = "build-flavor")]
    pub build_flavor: Option<String>,
    /// The --target-platform argument passed to 'flutter build'
    #[arg(long = "build-target-platform")]
    pub build_target_platform: Option<String>,
    /// The --export-options-plist argument passed 'flutter build'
    #[arg(long = "build-export-options-plist")]
    pub build_export_options_plist: Option<String>,
    /// The --dart-define argument(s) passed to 'flutter build'
    /// You may add multiple '--build-dart-define key=value' pairs
    #[arg(long = "build-dart-define", value_name = "foo=bar")]
    pub build_dart_define: Vec<String>,

    /// Shell command to run before packaging.
    #[arg(long = "hook-pre")]
    pub hook_pre: Option<String>,

    /// Shell command to run after packaging.
    #[arg(long = "hook-post")]
    pub hook_post: Option<String>,

    /// Package on a remote host from `~/.fastforge/hosts.yaml` and fetch the
    /// artifacts back. `auto` picks the first host whose `platforms` lists the
    /// platform.
    #[arg(long = "host", value_name = "NAME|auto")]
    pub host: Option<String>,
}

impl PackageArgs {
    /// Builds the `flutter build` argument map, mirroring Dart's
    /// `CommandPackage._generateBuildArgs`.
    fn build_arguments(&self) -> Map<String, Value> {
        let mut build_args = Map::new();
        if let Some(value) = &self.build_target {
            build_args.insert("target".to_string(), Value::String(value.clone()));
        }
        if let Some(value) = &self.build_flavor {
            build_args.insert("flavor".to_string(), Value::String(value.clone()));
        }
        if let Some(value) = &self.build_target_platform {
            build_args.insert("target-platform".to_string(), Value::String(value.clone()));
        }
        if let Some(value) = &self.build_export_options_plist {
            build_args.insert(
                "export-options-plist".to_string(),
                Value::String(value.clone()),
            );
        }
        if !self.build_dart_define.is_empty() {
            let mut defines = Map::new();
            for item in &self.build_dart_define {
                if let Some((key, value)) = item.split_once('=') {
                    defines.insert(key.to_string(), Value::String(value.to_string()));
                }
            }
            build_args.insert("dart-define".to_string(), Value::Object(defines));
        }
        for arg in self
            .flutter_build_args
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            match arg.split_once('=') {
                Some((key, value)) => {
                    build_args
                        .entry(key.to_string())
                        .or_insert(Value::String(value.to_string()));
                }
                None => {
                    build_args
                        .entry(arg.to_string())
                        .or_insert(Value::Bool(true));
                }
            }
        }
        build_args
    }
}

pub async fn execute(args: &PackageArgs) -> Result<()> {
    let targets: Vec<String> = args
        .targets
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    if targets.is_empty() {
        return Err(anyhow!("At least one 'target' must be specified!"));
    }
    let target_refs: Vec<&str> = targets.iter().map(String::as_str).collect();
    if let Some(host) = args.host.as_deref() {
        return super::remote::package(args, host, &target_refs);
    }
    let platform = match args.platform.as_deref() {
        Some(platform) => platform.to_string(),
        None => super::platform_infer::infer_platform(&target_refs)?
            .as_str()
            .to_string(),
    };

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

    // Like Dart, `package` honours `distribute_options.yaml`: its `output`
    // directory and its `variables` (layered over the environment). An explicit
    // `--output` wins over the file, so a workflow can drive the output
    // directory without one.
    let options = DistributeOptions::load()?;
    super::remote::hint_if_unsupported(&platform);
    let output = resolve_output(args.output.as_deref(), &options);
    package(PackageRequest {
        platform: &platform,
        targets: &targets,
        channel: args.channel.clone(),
        artifact_name: args.artifact_name.clone(),
        clean_before_build: !args.skip_clean,
        build_arguments: args.build_arguments(),
        variables: global_variables(&options),
        hooks: hooks.as_ref(),
        output: &output,
    })?;
    Ok(())
}

/// The directory the artifacts are written to: the `--output` argument when it
/// is given, the `output` key of `distribute_options.yaml` otherwise (which
/// itself defaults to `dist/` when the file does not exist).
pub(crate) fn resolve_output(argument: Option<&str>, options: &DistributeOptions) -> String {
    match argument.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => value.to_string(),
        None => options.output.clone(),
    }
}

/// Arguments of one packaging run (Dart's `UnifiedDistributor.package`).
pub struct PackageRequest<'a> {
    pub platform: &'a str,
    pub targets: &'a [String],
    pub channel: Option<String>,
    pub artifact_name: Option<String>,
    pub clean_before_build: bool,
    pub build_arguments: Map<String, Value>,
    pub variables: HashMap<String, String>,
    pub hooks: Option<&'a HashMap<String, serde_yaml::Value>>,
    pub output: &'a str,
}

/// The artifacts produced for one target.
pub struct PackagedTarget {
    pub artifacts: Vec<PathBuf>,
}

/// Packages the project for every requested target, mirroring Dart's
/// `UnifiedDistributor.package`: `flutter clean` at most once, non-android
/// platforms build once and reuse the output for every target, and a target
/// whose builder can't run on this host is skipped with a warning.
pub fn package(request: PackageRequest) -> Result<Vec<PackagedTarget>> {
    std::fs::create_dir_all(request.output)
        .with_context(|| format!("Failed to create {}", request.output))?;

    let mut results = Vec::new();
    let environment = request.variables;

    if !is_flutter_project() {
        for target in request.targets {
            println!("Packaging as {}:", target);
            let artifacts = match request.platform {
                "macos" => {
                    log::info!("Detected native macOS Xcode project (no pubspec.yaml)");
                    package_native_macos_artifact(
                        target,
                        request.build_arguments.clone(),
                        environment.clone(),
                        request.output,
                        request.artifact_name.clone(),
                        request.hooks,
                    )?
                }
                "ios" => {
                    log::info!("Detected native iOS Xcode project (no pubspec.yaml)");
                    package_native_ios_artifact(
                        target,
                        request.build_arguments.clone(),
                        environment.clone(),
                        request.output,
                        request.artifact_name.clone(),
                        request.hooks,
                    )?
                }
                "android" => {
                    log::info!("Detected native Android project (no pubspec.yaml)");
                    package_native_android_artifact(
                        target,
                        request.build_arguments.clone(),
                        environment.clone(),
                        request.output,
                        request.artifact_name.clone(),
                        request.hooks,
                    )?
                }
                other => {
                    return Err(anyhow!(
                        "No pubspec.yaml found: `{}` projects must be Flutter projects.",
                        other
                    ));
                }
            };
            results.push(PackagedTarget { artifacts });
        }
        return Ok(results);
    }

    let platform = Platform::from_str(request.platform)
        .map_err(|e| anyhow!("Invalid platform '{}': {}", request.platform, e))?;
    let pubspec = ProjectPubspec::load()?;
    let builder = FlutterAppBuilder::default();
    if request.clean_before_build {
        builder
            .clean(Some(&environment))
            .map_err(|e| anyhow!("{}", e))?;
    }

    let build_only_once = platform != Platform::Android;
    let mut cached_build: Option<BuildResult> = None;

    for target in request.targets {
        println!(
            "Packaging {} {} as {}:",
            pubspec.name, pubspec.version, target
        );

        // Reject unknown (platform, target) pairs before building. A builder
        // that can't run on this host is skipped with a warning (Dart
        // catches the builder's `UnsupportedError`); a packager that can't
        // run here fails before the expensive build.
        let packager = resolve_packager(platform, target)?;
        if builder.is_supported_on_current_platform(&platform, Some(target)) == Some(false) {
            eprintln!(
                "{}",
                yellow(&format!(
                    "Warning: {} is not supported on the current platform",
                    platform.as_str()
                ))
            );
            continue;
        }
        if !packager.is_supported_on_current_platform() {
            return Err(anyhow!(
                "Packager '{}' is not supported on the current platform",
                target
            ));
        }
        drop(packager);

        if !build_only_once || cached_build.is_none() {
            match builder.build(
                &platform,
                Some(target),
                request.build_arguments.clone(),
                Some(environment.clone()),
            ) {
                Ok(build) => {
                    print_build_result(&build)?;
                    cached_build = Some(build);
                }
                Err(BuildError::UnsupportedPlatform(message)) => {
                    eprintln!("{}", yellow(&format!("Warning: {}", message)));
                    continue;
                }
                Err(error) => return Err(anyhow!("{}", error)),
            }
        }

        let Some(build) = cached_build.as_ref() else {
            continue;
        };
        let artifacts = package_flutter_build(
            &platform,
            target,
            build,
            environment.clone(),
            request.output,
            request.artifact_name.clone(),
            request.channel.clone(),
            request.hooks,
        )?;
        results.push(PackagedTarget { artifacts });
    }
    Ok(results)
}

/// Prints a build result like Dart: the result JSON, then
/// `Successfully built <dir> in <n>s`.
fn print_build_result(build: &BuildResult) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&build.to_json_compatible())?
    );
    println!(
        "{}",
        bright_green(&format!(
            "Successfully built {} in {}s",
            build.output_directory.display(),
            build.duration_ms / 1000
        ))
    );
    Ok(())
}

/// Packages a single flutter target (clean → build → package). Used by the
/// local workflow runner.
#[allow(clippy::too_many_arguments)]
pub fn package_flutter_artifact(
    platform_str: &str,
    target: &str,
    build_args: Map<String, Value>,
    environment: HashMap<String, String>,
    output: &str,
    artifact_name: Option<String>,
    channel: Option<String>,
    clean_before_build: bool,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Vec<PathBuf>> {
    let targets = [target.to_string()];
    let results = package(PackageRequest {
        platform: platform_str,
        targets: &targets,
        channel,
        artifact_name,
        clean_before_build,
        build_arguments: build_args,
        variables: environment,
        hooks,
        output,
    })?;
    Ok(results
        .into_iter()
        .flat_map(|result| result.artifacts)
        .collect())
}

/// Packages an existing flutter build output as `target`.
#[allow(clippy::too_many_arguments)]
pub fn package_flutter_build(
    platform: &Platform,
    target: &str,
    build: &BuildResult,
    environment: HashMap<String, String>,
    output: &str,
    artifact_name: Option<String>,
    channel: Option<String>,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Vec<PathBuf>> {
    let platform = *platform;
    let packager = resolve_packager(platform, target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }

    let pubspec = ProjectPubspec::load()?;
    let app_binary_name = if platform == Platform::Linux {
        linux_binary_name().unwrap_or_else(|| pubspec.name.clone())
    } else {
        pubspec.name.clone()
    };
    let package_config = PackageConfig {
        app_name: pubspec.name.clone(),
        app_binary_name,
        app_version: pubspec.version,
        build_mode: build.config.mode().as_str().to_string(),
        platform,
        flavor: build.config.flavor().map(ToOwned::to_owned),
        channel,
        artifact_name,
        package_format: if target == "direct" {
            String::new()
        } else {
            target.to_string()
        },
        is_installer: is_installer_target(target),
        build_output_dir: build.output_directory.clone(),
        build_output_files: build.output_files.clone(),
        output_dir: PathBuf::from(output),
        environment: environment.clone(),
    };

    package_with_hooks(packager.as_ref(), &package_config, hooks, environment)
}

/// Runs the pre-package hooks, the packager and the post-package hooks, then
/// prints the result like Dart (`MakeResult` JSON and
/// `Successfully packaged <artifact>`).
fn package_with_hooks(
    packager: &(dyn AppPackager + Send + Sync),
    package_config: &PackageConfig,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
    environment: HashMap<String, String>,
) -> Result<Vec<PathBuf>> {
    let package_format = packager.package_format().to_string();
    let pre_hooks = resolve_hooks(hooks, "pre");
    let post_hooks = resolve_hooks(hooks, "post");

    let mut hook_env = environment;
    // Hooks also get the packaging variables (app metadata and the
    // `.fastforge/config.yaml` `env:` entries). Only read the project config
    // when there is a hook to run, so a broken config cannot fail packaging
    // formats that never use it.
    if !pre_hooks.is_empty() || !post_hooks.is_empty() {
        let settings = ProjectSettings::load().map_err(|e| anyhow!("{}", e))?;
        let variables = package_config.package_variables(&settings);
        hook_env
            .extend(environment_variables(&variables).map(|(k, v)| (k.to_string(), v.to_string())));
    }
    hook_env.insert(
        "PLATFORM".to_string(),
        package_config.platform.as_str().to_string(),
    );
    hook_env.insert("PACKAGE_FORMAT".to_string(), package_format.clone());
    hook_env.insert("BUILD_MODE".to_string(), package_config.build_mode.clone());
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

    run_hooks(&pre_hooks, &hook_env)?;
    let result = packager
        .package(package_config)
        .map_err(|e| anyhow!("{}", e))?;
    run_hooks(&post_hooks, &hook_env)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&make_result_json(
            package_config,
            &package_format,
            &result.artifacts
        ))?
    );
    if let Some(artifact) = result.artifacts.first() {
        println!(
            "{}",
            bright_green(&format!("Successfully packaged {}", artifact.display()))
        );
    }
    Ok(result.artifacts)
}

/// Dart's `MakeResult.toJson()` (`config` is `MakeConfig.toJson()`).
fn make_result_json(config: &PackageConfig, package_format: &str, artifacts: &[PathBuf]) -> Value {
    let mut make_config = Map::new();
    let mut put = |key: &str, value: Value| {
        make_config.insert(key.to_string(), value);
    };
    put("isInstaller", Value::Bool(config.is_installer));
    put("buildMode", Value::String(config.build_mode.clone()));
    put(
        "buildOutputDirectory",
        Value::String(config.build_output_dir.to_string_lossy().to_string()),
    );
    put(
        "buildOutputFiles",
        Value::Array(
            config
                .build_output_files
                .iter()
                .map(|p| Value::String(p.to_string_lossy().to_string()))
                .collect(),
        ),
    );
    put(
        "platform",
        Value::String(config.platform.as_str().to_string()),
    );
    if let Some(flavor) = &config.flavor {
        put("flavor", Value::String(flavor.clone()));
    }
    if let Some(channel) = &config.channel {
        put("channel", Value::String(channel.clone()));
    }
    if let Some(artifact_name) = &config.artifact_name {
        put("artifactName", Value::String(artifact_name.clone()));
    }
    put("packageFormat", Value::String(package_format.to_string()));
    put(
        "outputDirectory",
        Value::String(config.output_dir.to_string_lossy().to_string()),
    );
    put("appName", Value::String(config.app_name.clone()));
    put("appVersion", Value::String(config.app_version.clone()));
    put(
        "appBuildName",
        Value::String(
            config
                .app_version
                .split('+')
                .next()
                .unwrap_or_default()
                .to_string(),
        ),
    );
    put(
        "appBuildNumber",
        Value::String(
            config
                .app_version
                .rsplit('+')
                .next()
                .unwrap_or_default()
                .to_string(),
        ),
    );

    let artifacts: Vec<Value> = artifacts
        .iter()
        .map(|path| {
            serde_json::json!({
                "type": if path.is_dir() { "directory" } else { "file" },
                "path": path.to_string_lossy(),
            })
        })
        .collect();
    serde_json::json!({ "config": make_config, "artifacts": artifacts })
}

/// Extract and normalize hook commands for a given key ("pre" or "post").
/// Supports both a single string and a list of strings.
fn resolve_hooks(hooks: Option<&HashMap<String, serde_yaml::Value>>, key: &str) -> Vec<String> {
    let Some(hooks) = hooks else { return vec![] };
    let Some(value) = hooks.get(key) else {
        return vec![];
    };
    match value {
        serde_yaml::Value::String(cmd) => vec![cmd.clone()],
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

/// Executes hook commands with `sh -c`, echoing and streaming their output
/// like Dart's shell executor.
fn run_hooks(hooks: &[String], env: &HashMap<String, String>) -> Result<()> {
    for hook in hooks {
        let mut command = Command::new("sh");
        command.args(["-c", hook]).envs(env);
        let (status, stderr) = run_streaming(&mut command, &format!("sh -c {}", hook))?;
        if !status.success() {
            return Err(anyhow!(
                "Hook failed (exit {}): {}\n{}",
                status.code().unwrap_or(-1),
                hook,
                stderr,
            ));
        }
    }
    Ok(())
}

/// Resolves the packager for a `(platform, target)` pair, covering the same
/// matrix as Dart's `FlutterAppPackager` maker registry.
pub fn resolve_packager(
    platform: Platform,
    target: &str,
) -> Result<Box<dyn AppPackager + Send + Sync>> {
    if target == "custom" {
        return Ok(Box::new(CustomPackager::load(platform)?));
    }
    match (platform, target) {
        (Platform::Android, "aab") => Ok(Box::new(AndroidAabPackager)),
        (Platform::Android, "apk") => Ok(Box::new(AndroidApkPackager)),
        (Platform::IOS, "ipa") => Ok(Box::new(IOSIpaPackager)),
        (Platform::Linux, "appimage") => Ok(Box::new(LinuxAppImagePackager)),
        (Platform::Linux, "deb") => Ok(Box::new(LinuxDebPackager)),
        (Platform::Linux, "pacman") => Ok(Box::new(LinuxPacmanPackager)),
        (Platform::Linux, "rpm") => Ok(Box::new(LinuxRpmPackager)),
        (Platform::Linux, "zip") => Ok(Box::new(LinuxZipPackager)),
        (Platform::Linux, "direct") => Ok(Box::new(LinuxDirectPackager)),
        (Platform::MacOS, "pkg") => Ok(Box::new(MacOSPkgPackager::from_yaml_file(
            std::path::Path::new("macos/packaging/pkg/make_config.yaml"),
        )?)),
        (Platform::MacOS, "dmg") => Ok(Box::new(MacOSDmgPackager)),
        (Platform::MacOS, "zip") => Ok(Box::new(MacOSZipPackager)),
        (Platform::Ohos, "app") => Ok(Box::new(OHOSAppPackager)),
        (Platform::Ohos, "hap") => Ok(Box::new(OHOSHapPackager)),
        (Platform::Web, "zip") => Ok(Box::new(WebZipPackager)),
        (Platform::Web, "direct") => Ok(Box::new(WebDirectPackager)),
        (Platform::Windows, "exe") => Ok(Box::new(WindowsExePackager)),
        (Platform::Windows, "msix") => Ok(Box::new(WindowsMsixPackager::default())),
        (Platform::Windows, "zip") => Ok(Box::new(WindowsZipPackager)),
        (Platform::Windows, "direct") => Ok(Box::new(WindowsDirectPackager)),
        (platform, other) => Err(anyhow!(
            "Unsupported package target `{}` for platform `{}`.",
            other,
            platform.as_str(),
        )),
    }
}

/// Whether packaging as `target` produces an installer artifact.
/// Mirrors Dart, where only the `exe` maker sets `isInstaller = true`
/// (reflected in the `-setup` suffix of the default artifact name).
fn is_installer_target(target: &str) -> bool {
    target == "exe"
}

/// Reads `BINARY_NAME` from `linux/CMakeLists.txt`, mirroring Dart's
/// `MakeLinuxPackageConfig.appBinaryName`.
fn linux_binary_name() -> Option<String> {
    let content = std::fs::read_to_string("linux/CMakeLists.txt").ok()?;
    let start = content.find("set(BINARY_NAME \"")? + "set(BINARY_NAME \"".len();
    let rest = &content[start..];
    let end = rest.find('"')?;
    let name = &rest[..end];
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn macos_packager(target: &str) -> Result<Box<dyn AppPackager + Send + Sync>> {
    resolve_packager(Platform::MacOS, target)
}

fn ios_packager(target: &str) -> Result<Box<dyn AppPackager + Send + Sync>> {
    match target {
        "ipa" => Ok(Box::new(IOSIpaPackager)),
        other => Err(anyhow!(
            "Unsupported iOS package target: `{}`. Currently supported: ipa",
            other
        )),
    }
}

fn android_packager(target: &str) -> Result<Box<dyn AppPackager + Send + Sync>> {
    match target {
        "aab" => Ok(Box::new(AndroidAabPackager)),
        "apk" => Ok(Box::new(AndroidApkPackager)),
        other => Err(anyhow!(
            "Unsupported Android package target: `{}`. Currently supported: aab, apk",
            other
        )),
    }
}

// ─── Native macOS Xcode project support ───────────────────────────────────

/// Package a native macOS Xcode project into the specified format.
///
/// Unlike `package_flutter_artifact`, this function:
/// - Uses `xcodebuild` to build the `.app` (via `MacOSXcodeAppBuilder`).
/// - Reads app metadata from `Info.plist` in the built `.app`.
/// - Supports all macOS packagers (pkg, dmg, zip).
#[allow(clippy::too_many_arguments)]
pub fn package_native_macos_artifact(
    target: &str,
    build_args: Map<String, Value>,
    environment: HashMap<String, String>,
    output: &str,
    artifact_name: Option<String>,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Vec<PathBuf>> {
    // Build the Xcode project
    let xcode_builder = MacOSXcodeAppBuilder::default();
    let build = xcode_builder
        .build(
            "macos",
            Some("macos-xcode"),
            build_args.clone(),
            Some(environment.clone()),
        )
        .map_err(|e| anyhow!("Xcode build failed: {}", e))?;
    print_build_result(&build)?;

    // Read metadata from the built .app's Info.plist
    let app_path = build
        .output_files
        .first()
        .ok_or_else(|| anyhow!("No .app bundle produced by Xcode build"))?;

    // Read name, version, build number from Info.plist
    let (app_name, version, build_number) = read_native_macos_metadata(app_path)?;
    let app_version = format!("{}+{}", version, build_number);

    let package_config = PackageConfig {
        app_name: app_name.clone(),
        app_binary_name: app_name,
        app_version,
        build_mode: "release".to_string(),
        platform: Platform::MacOS,
        flavor: None,
        channel: None,
        artifact_name,
        package_format: target.to_string(),
        is_installer: is_installer_target(target),
        build_output_dir: build.output_directory,
        build_output_files: build.output_files,
        output_dir: PathBuf::from(output),
        environment: environment.clone(),
    };

    let packager = macos_packager(target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }

    package_with_hooks(packager.as_ref(), &package_config, hooks, environment)
}

/// Read metadata (name, version, build_number) from a built macOS .app bundle.
fn read_native_macos_metadata(app_path: &std::path::Path) -> Result<(String, String, String)> {
    let plist_path = app_path.join("Contents").join("Info.plist");
    if !plist_path.exists() {
        return Err(anyhow!("Info.plist not found at {}", plist_path.display()));
    }

    let name = plutil_read(&plist_path, "CFBundleName")?;
    let version = plutil_read(&plist_path, "CFBundleShortVersionString")
        .unwrap_or_else(|_| "0.1.0".to_string());
    let build_number =
        plutil_read(&plist_path, "CFBundleVersion").unwrap_or_else(|_| "1".to_string());

    Ok((name, version, build_number))
}

/// Extract a value from a plist file using `plutil`.
fn plutil_read(plist_path: &std::path::Path, key: &str) -> Result<String> {
    let out = std::process::Command::new("plutil")
        .args([
            "-extract",
            key,
            "raw",
            "-o",
            "-",
            &plist_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| anyhow!("plutil: {}", e))?;
    if !out.status.success() {
        return Err(anyhow!("Failed to read `{}` from Info.plist", key));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ─── Native iOS Xcode project support ─────────────────────────────────────

/// Package a native iOS Xcode project into the specified format.
///
/// Unlike `package_flutter_artifact`, this function:
/// - Uses `xcodebuild archive` + `xcodebuild -exportArchive` (via `IOSXcodeAppBuilder`).
/// - Reads app metadata from `Info.plist` in the built `.app`.
/// - Supports the ipa packager.
#[allow(clippy::too_many_arguments)]
pub fn package_native_ios_artifact(
    target: &str,
    build_args: Map<String, Value>,
    environment: HashMap<String, String>,
    output: &str,
    artifact_name: Option<String>,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Vec<PathBuf>> {
    // Ensure target is ipa
    if target != "ipa" {
        return Err(anyhow!(
            "Native iOS packaging only supports 'ipa' target, got '{}'",
            target
        ));
    }

    // Build and export the iOS app
    let xcode_builder = IOSXcodeAppBuilder::default();
    let build = xcode_builder
        .build(
            "ios",
            Some("ios-xcode"),
            build_args.clone(),
            Some(environment.clone()),
        )
        .map_err(|e| anyhow!("iOS Xcode build failed: {}", e))?;
    print_build_result(&build)?;

    // Read metadata from the built app.
    // First try: find the .app inside the .xcarchive (Products/Applications/<App>.app).
    // Fallback: extract Info.plist from the generated IPA.
    let (app_name, version, build_number) = 'meta: {
        // Try archive path first
        if let Some(archive_path_str) = build_args.get("archive-path").and_then(|v| v.as_str()) {
            let app_dir = std::path::Path::new(archive_path_str)
                .join("Products")
                .join("Applications");
            if let Ok(entries) = std::fs::read_dir(&app_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "app")
                        && let Ok(meta) = read_native_macos_metadata(&path)
                    {
                        break 'meta meta;
                    }
                }
            }
        }
        // Fallback: read from the IPA
        if let Some(ipa_path) = build.output_files.first()
            && let Ok(meta) = read_app_name_from_ipa(ipa_path)
        {
            break 'meta meta;
        }
        ("Runner".to_string(), "0.1.0".to_string(), "1".to_string())
    };

    let app_version = format!("{}+{}", version, build_number);

    let package_config = PackageConfig {
        app_name: app_name.clone(),
        app_binary_name: app_name,
        app_version,
        build_mode: "release".to_string(),
        platform: Platform::IOS,
        flavor: None,
        channel: None,
        artifact_name,
        package_format: target.to_string(),
        is_installer: false,
        build_output_dir: build.output_directory,
        build_output_files: build.output_files,
        output_dir: PathBuf::from(output),
        environment: environment.clone(),
    };

    let packager = ios_packager(target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }

    package_with_hooks(packager.as_ref(), &package_config, hooks, environment)
}

/// Attempt to read the app name and version from an IPA's embedded Info.plist.
/// Uses `unzip` to extract `Payload/*.app/Info.plist` and `plutil` to parse it.
fn read_app_name_from_ipa(ipa_path: &std::path::Path) -> Result<(String, String, String)> {
    // Create a temporary directory for extraction
    let tmp_dir = std::env::temp_dir().join(format!(
        "fastforge_ipa_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp_dir).ok();

    // Extract Info.plist from the IPA
    let status = std::process::Command::new("unzip")
        .args([
            "-o",
            &ipa_path.to_string_lossy(),
            "Payload/*.app/Info.plist",
            "-d",
            &tmp_dir.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| anyhow!("unzip failed: {}", e))?;

    if !status.success() {
        return Err(anyhow!("Failed to extract Info.plist from IPA"));
    }

    // Find the extracted Info.plist
    let find_result = std::process::Command::new("find")
        .args([&tmp_dir.to_string_lossy(), "-name", "Info.plist"])
        .output()
        .map_err(|e| anyhow!("find failed: {}", e))?;
    let plist_path_str = String::from_utf8_lossy(&find_result.stdout)
        .lines()
        .next()
        .map(|s| s.to_string());

    let plist_path = match plist_path_str {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            std::fs::remove_dir_all(&tmp_dir).ok();
            return Err(anyhow!("No Info.plist found in IPA"));
        }
    };

    let name = plutil_read(&plist_path, "CFBundleName").unwrap_or_else(|_| "Runner".to_string());
    let version = plutil_read(&plist_path, "CFBundleShortVersionString")
        .unwrap_or_else(|_| "0.1.0".to_string());
    let build_number =
        plutil_read(&plist_path, "CFBundleVersion").unwrap_or_else(|_| "1".to_string());

    // Clean up
    std::fs::remove_dir_all(&tmp_dir).ok();

    Ok((name, version, build_number))
}

// ─── Native Android project support ───────────────────────────────────

/// Package a native Android project into the specified format.
///
/// Uses `GradleAppBuilder` to run `./gradlew` tasks and reads app metadata
/// from `app/build.gradle.kts`.
#[allow(clippy::too_many_arguments)]
pub fn package_native_android_artifact(
    target: &str,
    build_args: Map<String, Value>,
    environment: HashMap<String, String>,
    output: &str,
    artifact_name: Option<String>,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Vec<PathBuf>> {
    if target != "aab" && target != "apk" {
        return Err(anyhow!(
            "Native Android packaging only supports 'aab' and 'apk' targets, got '{}'",
            target
        ));
    }

    let gradle_builder = GradleAppBuilder::default();
    let build = gradle_builder
        .build(
            "gradle-android",
            Some(target),
            build_args.clone(),
            Some(environment.clone()),
        )
        .map_err(|e| anyhow!("Gradle build failed: {}", e))?;
    print_build_result(&build)?;

    // Read metadata from app/build.gradle.kts
    let version_info = read_android_metadata()?;
    let app_version = format!("{}+{}", version_info.1, version_info.2);

    let package_config = PackageConfig {
        app_name: version_info.0.clone(),
        app_binary_name: version_info.0,
        app_version,
        build_mode: "release".to_string(),
        platform: Platform::Android,
        flavor: None,
        channel: None,
        artifact_name,
        package_format: target.to_string(),
        is_installer: false,
        build_output_dir: build.output_directory,
        build_output_files: build.output_files,
        output_dir: PathBuf::from(output),
        environment: environment.clone(),
    };

    let packager = android_packager(target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }

    package_with_hooks(packager.as_ref(), &package_config, hooks, environment)
}

/// Read app name and version info from `app/build.gradle.kts`.
fn read_android_metadata() -> Result<(String, String, String)> {
    let content = std::fs::read_to_string("app/build.gradle.kts")
        .context("Failed to read app/build.gradle.kts")?;

    let application_id = content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("applicationId") {
                trimmed
                    .split('=')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "app".to_string());

    let version_name = content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("versionName") {
                trimmed
                    .split('=')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "0.1.0".to_string());

    let version_code = content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("versionCode") {
                trimmed.split('=').nth(1).map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "1".to_string());

    Ok((application_id, version_name, version_code))
}

/// Check whether the current working directory contains a Flutter project
/// (i.e., has a pubspec.yaml file).
pub fn is_flutter_project() -> bool {
    std::path::Path::new("pubspec.yaml").exists()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_argument_wins_over_distribute_options() {
        let options = DistributeOptions {
            output: "from-file/".to_string(),
            ..DistributeOptions::default()
        };

        assert_eq!(resolve_output(Some("from-cli/"), &options), "from-cli/");
        // An empty or blank argument falls back to the file (and to `dist/`
        // when there is no file).
        assert_eq!(resolve_output(Some(""), &options), "from-file/");
        assert_eq!(resolve_output(Some("   "), &options), "from-file/");
        assert_eq!(resolve_output(None, &options), "from-file/");
        assert_eq!(resolve_output(None, &DistributeOptions::default()), "dist/");
    }

    /// The full (platform, target) matrix registered by Dart's
    /// `FlutterAppPackager` (minus `custom`, which needs a config file).
    #[test]
    fn resolve_packager_covers_dart_maker_matrix() {
        let matrix: &[(&str, &[&str])] = &[
            ("android", &["aab", "apk"]),
            ("ios", &["ipa"]),
            (
                "linux",
                &["appimage", "deb", "pacman", "rpm", "zip", "direct"],
            ),
            ("macos", &["dmg", "pkg", "zip"]),
            ("ohos", &["app", "hap"]),
            ("web", &["zip", "direct"]),
            ("windows", &["exe", "msix", "zip", "direct"]),
        ];
        for (platform_str, targets) in matrix {
            let platform = Platform::from_str(platform_str).unwrap();
            for target in *targets {
                let packager = resolve_packager(platform, target).unwrap_or_else(|e| {
                    panic!("resolve_packager({platform_str}, {target}) failed: {e}")
                });
                assert_eq!(packager.name(), *target);
            }
        }
    }

    #[test]
    fn resolve_packager_rejects_mismatched_pairs() {
        for (platform_str, target) in [
            ("macos", "apk"),
            ("android", "dmg"),
            ("linux", "msix"),
            ("web", "deb"),
        ] {
            let platform = Platform::from_str(platform_str).unwrap();
            match resolve_packager(platform, target) {
                Ok(_) => panic!("({platform_str}, {target}) must be rejected"),
                Err(err) => assert!(
                    err.to_string().contains("Unsupported package target"),
                    "unexpected error for ({platform_str}, {target}): {err}"
                ),
            }
        }
    }

    #[test]
    fn only_exe_is_an_installer_target() {
        assert!(is_installer_target("exe"));
        for target in ["dmg", "pkg", "deb", "rpm", "pacman", "msix", "apk", "zip"] {
            assert!(
                !is_installer_target(target),
                "{target} must not be an installer"
            );
        }
    }
}
