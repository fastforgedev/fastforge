use anyhow::{Context, Result, anyhow};
use clap::Args;
use fastforge_app_builder::{
    FlutterAppBuilder, GradleAppBuilder, IOSXcodeAppBuilder, MacOSXcodeAppBuilder, Platform,
};
use fastforge_app_packager::{
    AndroidAabPackager, AndroidApkPackager, AppPackager, CustomPackager, IOSIpaPackager,
    LinuxAppImagePackager, LinuxDebPackager, LinuxDirectPackager, LinuxPacmanPackager,
    LinuxRpmPackager, LinuxZipPackager, MacOSDmgPackager, MacOSPkgPackager, MacOSZipPackager,
    OHOSAppPackager, OHOSHapPackager, PackageConfig, WebDirectPackager, WebZipPackager,
    WindowsDirectPackager, WindowsExePackager, WindowsMsixPackager, WindowsZipPackager,
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
    /// Target platform (auto-detected from the targets and project layout
    /// when omitted).
    #[arg(short, long = "platform")]
    pub platform: Option<String>,
    /// Comma-separated list of bundle types to build (e.g. `apk`, `dmg,zip`).
    #[arg(short, long = "targets", alias = "target", value_name = "TARGET,...")]
    pub targets: Option<String>,
    /// Release channel included in the artifact name.
    #[arg(long = "channel")]
    pub channel: Option<String>,
    /// Artifact name template (mustache syntax, e.g. `{{name}}-{{build_name}}.{{ext}}`).
    #[arg(long = "artifact-name")]
    pub artifact_name: Option<String>,
    #[arg(long = "skip-clean", default_value_t = false)]
    pub skip_clean: bool,

    /// Comma-separated arguments passed directly to `flutter build`
    /// (e.g. `verbose,obfuscate` or `split-debug-info=./symbols`).
    #[arg(long = "flutter-build-args", value_name = "ARG,...")]
    pub flutter_build_args: Option<String>,
    /// The --target argument passed to `flutter build`.
    #[arg(long = "build-target")]
    pub build_target: Option<String>,
    /// The --flavor argument passed to `flutter build`.
    #[arg(long = "build-flavor")]
    pub build_flavor: Option<String>,
    /// The --target-platform argument passed to `flutter build`.
    #[arg(long = "build-target-platform")]
    pub build_target_platform: Option<String>,
    /// The --export-options-plist argument passed to `flutter build`.
    #[arg(long = "build-export-options-plist")]
    pub build_export_options_plist: Option<String>,
    /// The --dart-define argument(s) passed to `flutter build`.
    /// May be repeated: `--build-dart-define foo=bar --build-dart-define a=b`.
    #[arg(long = "build-dart-define", value_name = "KEY=VALUE")]
    pub build_dart_define: Vec<String>,

    /// Shell command to run before packaging.
    #[arg(long = "hook-pre")]
    pub hook_pre: Option<String>,

    /// Shell command to run after packaging.
    #[arg(long = "hook-post")]
    pub hook_post: Option<String>,
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
                    build_args.entry(arg.to_string()).or_insert(Value::Bool(true));
                }
            }
        }
        build_args
    }
}

pub async fn execute(args: &PackageArgs) -> Result<()> {
    log::info!("Executing package command");
    let targets: Vec<&str> = args
        .targets
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if targets.is_empty() {
        return Err(anyhow!("At least one 'target' must be specified!"));
    }
    let platform = match args.platform.as_deref() {
        Some(platform) => platform.to_string(),
        None => super::platform_infer::infer_platform(&targets)?
            .as_str()
            .to_string(),
    };
    let platform = platform.as_str();

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
    let mut clean_before_build = !args.skip_clean;
    let mut artifacts = Vec::new();

    // Non-android flutter platforms build once and reuse the output for
    // subsequent targets (mirrors Dart's `isBuildOnlyOnce`); android rebuilds
    // per target because apk/aab need different `flutter build` subcommands.
    let mut cached_build: Option<fastforge_app_builder::BuildResult> = None;

    for target in targets {
        let build_args = args.build_arguments();
        let target_artifacts = if is_native && platform == "macos" {
            log::info!("Detected native macOS Xcode project (no pubspec.yaml)");
            package_native_macos_artifact(
                target,
                build_args,
                std::env::vars().collect(),
                "dist/",
                args.artifact_name.clone(),
                hooks.as_ref(),
            )?
        } else if is_native && platform == "ios" {
            log::info!("Detected native iOS Xcode project (no pubspec.yaml)");
            package_native_ios_artifact(
                target,
                build_args,
                std::env::vars().collect(),
                "dist/",
                args.artifact_name.clone(),
                hooks.as_ref(),
            )?
        } else if is_native && platform == "android" {
            log::info!("Detected native Android project (no pubspec.yaml)");
            package_native_android_artifact(
                target,
                build_args,
                std::env::vars().collect(),
                "dist/",
                args.artifact_name.clone(),
                hooks.as_ref(),
            )?
        } else {
            let flutter_platform = Platform::from_str(platform)
                .map_err(|e| anyhow!("Invalid platform '{}': {}", platform, e))?;

            // Fail fast on unsupported (platform, target) pairs before building.
            let packager = resolve_packager(flutter_platform, target)?;
            if !packager.is_supported_on_current_platform() {
                return Err(anyhow!(
                    "Packager '{}' is not supported on the current platform",
                    target
                ));
            }
            drop(packager);

            let environment: HashMap<String, String> = std::env::vars().collect();
            let build = match (&cached_build, flutter_platform) {
                (Some(build), p) if p != Platform::Android => build,
                _ => {
                    let build = build_flutter_target(
                        &flutter_platform,
                        target,
                        build_args,
                        &environment,
                        clean_before_build,
                    )?;
                    cached_build = Some(build);
                    cached_build.as_ref().unwrap()
                }
            };

            package_flutter_build(
                &flutter_platform,
                target,
                build,
                environment,
                "dist/",
                args.artifact_name.clone(),
                args.channel.clone(),
                hooks.as_ref(),
            )?
        };
        // Clean at most once per invocation (mirrors Dart).
        clean_before_build = false;

        // Print a JSON summary per packaged target (mirrors Dart's
        // MakeResult JSON output).
        let summary = serde_json::json!({
            "platform": platform,
            "target": target,
            "artifacts": target_artifacts
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);

        artifacts.extend(target_artifacts);
    }

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
    channel: Option<String>,
    clean_before_build: bool,
    hooks: Option<&HashMap<String, serde_yaml::Value>>,
) -> Result<Vec<PathBuf>> {
    let platform = Platform::from_str(platform_str)
        .map_err(|e| anyhow!("Invalid platform '{}': {}", platform_str, e))?;

    // Resolve the packager up-front so unsupported (platform, target) pairs
    // fail fast, before any expensive build step runs.
    let packager = resolve_packager(platform, target)?;
    if !packager.is_supported_on_current_platform() {
        return Err(anyhow!(
            "Packager '{}' is not supported on the current platform",
            target
        ));
    }
    drop(packager);

    let build = build_flutter_target(&platform, target, build_args, &environment, clean_before_build)?;
    package_flutter_build(
        &platform,
        target,
        &build,
        environment,
        output,
        artifact_name,
        channel,
        hooks,
    )
}

/// Runs `flutter build` for a `(platform, target)` pair, optionally cleaning
/// first. Split from packaging so multi-target invocations can build once and
/// reuse the output (mirrors Dart's `isBuildOnlyOnce` behavior).
pub fn build_flutter_target(
    platform: &Platform,
    target: &str,
    build_args: Map<String, Value>,
    environment: &HashMap<String, String>,
    clean_before_build: bool,
) -> Result<fastforge_app_builder::BuildResult> {
    let builder = FlutterAppBuilder::default();
    if clean_before_build {
        builder
            .clean(Some(environment))
            .map_err(|e| anyhow!("{}", e))?;
    }
    builder
        .build(platform, Some(target), build_args, Some(environment.clone()))
        .map_err(|e| anyhow!("{}", e))
}

/// Packages an existing flutter build output as `target`.
#[allow(clippy::too_many_arguments)]
pub fn package_flutter_build(
    platform: &Platform,
    target: &str,
    build: &fastforge_app_builder::BuildResult,
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

    let hook_env_base = environment;

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
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The full (platform, target) matrix registered by Dart's
    /// `FlutterAppPackager` (minus `custom`, which needs a config file).
    #[test]
    fn resolve_packager_covers_dart_maker_matrix() {
        let matrix: &[(&str, &[&str])] = &[
            ("android", &["aab", "apk"]),
            ("ios", &["ipa"]),
            ("linux", &["appimage", "deb", "pacman", "rpm", "zip", "direct"]),
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
            assert!(!is_installer_target(target), "{target} must not be an installer");
        }
    }
}
