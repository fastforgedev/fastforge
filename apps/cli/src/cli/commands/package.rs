use anyhow::{Context, Result, anyhow};
use clap::Args;
use fastforge_app_builder::{
    FlutterAppBuilder, GradleAppBuilder, IOSXcodeAppBuilder, MacOSXcodeAppBuilder, Platform,
};
use fastforge_app_packager::{
    AndroidAabPackager, AndroidApkPackager, AppPackager, IOSIpaPackager, MacOSDmgPackager,
    MacOSPkgPackager, MacOSZipPackager, PackageConfig,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use serde_yaml;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
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

    let is_native = !is_flutter_project();

    let artifacts = if is_native && platform == "macos" {
        log::info!("Detected native macOS Xcode project (no pubspec.yaml)");
        package_native_macos_artifact(
            target,
            build_args,
            std::env::vars().collect(),
            "dist/",
            None,
            hooks.as_ref(),
        )?
    } else if is_native && platform == "ios" {
        log::info!("Detected native iOS Xcode project (no pubspec.yaml)");
        package_native_ios_artifact(
            target,
            build_args,
            std::env::vars().collect(),
            "dist/",
            None,
            hooks.as_ref(),
        )?
    } else if is_native && platform == "android" {
        log::info!("Detected native Android project (no pubspec.yaml)");
        package_native_android_artifact(
            target,
            build_args,
            std::env::vars().collect(),
            "dist/",
            None,
            hooks.as_ref(),
        )?
    } else {
        package_flutter_artifact(
            platform,
            target,
            build_args,
            std::env::vars().collect(),
            "dist/",
            None,
            !args.skip_clean,
            hooks.as_ref(),
        )?
    };

    for artifact in artifacts {
        println!("{}", artifact.display());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
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
        is_installer: matches!(target, "pkg" | "dmg"),
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

    // Resolve hooks
    let pre_hooks = resolve_hooks(hooks, "pre");
    let post_hooks = resolve_hooks(hooks, "post");

    // Build hook environment
    let mut hook_env = environment;
    hook_env.insert(
        "PLATFORM".to_string(),
        package_config.platform.as_str().to_string(),
    );
    hook_env.insert(
        "PACKAGE_FORMAT".to_string(),
        package_config.package_format.clone(),
    );
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

    // Run prepackage hooks
    run_hooks(&pre_hooks, &hook_env)?;

    let result = packager
        .package(&package_config)
        .map_err(|e| anyhow!("{}", e))?;

    // Run postpackage hooks
    run_hooks(&post_hooks, &hook_env)?;

    Ok(result.artifacts)
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
    };

    let packager = ios_packager(target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }

    // Resolve hooks
    let pre_hooks = resolve_hooks(hooks, "pre");
    let post_hooks = resolve_hooks(hooks, "post");

    // Build hook environment
    let mut hook_env = environment;
    hook_env.insert(
        "PLATFORM".to_string(),
        package_config.platform.as_str().to_string(),
    );
    hook_env.insert(
        "PACKAGE_FORMAT".to_string(),
        package_config.package_format.clone(),
    );
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

    // Run prepackage hooks
    run_hooks(&pre_hooks, &hook_env)?;

    let result = packager
        .package(&package_config)
        .map_err(|e| anyhow!("{}", e))?;

    // Run postpackage hooks
    run_hooks(&post_hooks, &hook_env)?;

    Ok(result.artifacts)
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
    };

    let packager = android_packager(target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }

    let pre_hooks = resolve_hooks(hooks, "pre");
    let post_hooks = resolve_hooks(hooks, "post");

    let mut hook_env = environment;
    hook_env.insert(
        "PLATFORM".to_string(),
        package_config.platform.as_str().to_string(),
    );
    hook_env.insert(
        "PACKAGE_FORMAT".to_string(),
        package_config.package_format.clone(),
    );
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
        .package(&package_config)
        .map_err(|e| anyhow!("{}", e))?;
    run_hooks(&post_hooks, &hook_env)?;

    Ok(result.artifacts)
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
