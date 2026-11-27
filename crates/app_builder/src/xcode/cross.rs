//! Cross-platform Xcode project builder powered by CrossXcodeBuild
//! (`libnativeapi/crossswift`).
//!
//! Compiles unmodified `.xcodeproj` / `.xcworkspace` projects for non-Apple
//! targets: Windows, Linux, WebAssembly (`web`), Android, and HarmonyOS
//! (`ohos`). Apple platforms (`macos`, `ios`) continue to use Apple's
//! `xcodebuild` via [`super::MacOSXcodeAppBuilder`] and
//! [`super::ios::IOSXcodeAppBuilder`].

use fastforge_core::{AppBuilder, BuildConfig, BuildError, BuildResult, Platform};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use super::helpers::{extract_optional_str, run};

/// Supported non-Apple target platforms for CrossXcodeBuild.
pub const CROSS_XCODE_PLATFORMS: [Platform; 5] = [
    Platform::Windows,
    Platform::Linux,
    Platform::Web,
    Platform::Android,
    Platform::Ohos,
];

/// Returns whether `platform` is supported by CrossXcodeBuild.
pub fn is_cross_xcode_platform(platform: Platform) -> bool {
    CROSS_XCODE_PLATFORMS.contains(&platform)
}

/// Default `-destination` value passed to CrossXcodeBuild for `platform`.
pub fn default_destination(platform: Platform) -> Option<&'static str> {
    match platform {
        Platform::Windows => Some("generic/platform=Windows"),
        Platform::Linux => Some("generic/platform=Linux"),
        Platform::Web => Some("generic/platform=Wasm"),
        Platform::Android => Some("generic/platform=Android"),
        Platform::Ohos => Some("generic/platform=HarmonyOS"),
        Platform::MacOS | Platform::IOS => None,
    }
}

/// Builder identifier for a CrossXcodeBuild target platform.
pub fn builder_name(platform: Platform) -> Option<&'static str> {
    match platform {
        Platform::Windows => Some("windows-xcode"),
        Platform::Linux => Some("linux-xcode"),
        Platform::Web => Some("web-xcode"),
        Platform::Android => Some("android-xcode"),
        Platform::Ohos => Some("ohos-xcode"),
        Platform::MacOS | Platform::IOS => None,
    }
}

/// Whether `host` can execute a CrossXcodeBuild build for `target`.
///
/// Mirrors `TargetPlatform.canBuildOnCurrentHost` in `libnativeapi/crossswift`:
/// Windows and Linux desktop binaries build natively on the matching host;
/// WebAssembly, Android, and HarmonyOS cross-compile from macOS or Linux.
pub fn is_host_supported(target: Platform, host: Option<Platform>) -> bool {
    match target {
        Platform::Windows => host == Some(Platform::Windows),
        Platform::Linux => host == Some(Platform::Linux),
        Platform::Web | Platform::Android | Platform::Ohos => {
            matches!(host, Some(Platform::MacOS | Platform::Linux))
        }
        Platform::MacOS | Platform::IOS => false,
    }
}

/// Resolved command invocation for CrossXcodeBuild (`libnativeapi/crossswift`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossXcodeBuildCommand {
    pub program: PathBuf,
    pub prefix_args: Vec<String>,
}

impl CrossXcodeBuildCommand {
    /// Creates a `std::process::Command` with the resolved program and prefix arguments.
    pub fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.prefix_args);
        cmd
    }
}

/// Metadata extracted from CrossXcodeBuild (`-showBuildSettings -json`) or
/// inferred from build arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossXcodeProjectMetadata {
    pub product_name: String,
    pub executable_name: String,
    pub marketing_version: String,
    pub build_version: Option<String>,
    pub bundle_identifier: Option<String>,
    pub built_products_dir: Option<PathBuf>,
}

impl CrossXcodeProjectMetadata {
    /// Formats the app version as `<marketing>+<build>` when a build version
    /// is present, otherwise `<marketing>`.
    pub fn app_version(&self) -> String {
        match self.build_version.as_deref().map(str::trim) {
            Some(build) if !build.is_empty() => format!("{}+{}", self.marketing_version, build),
            _ => self.marketing_version.clone(),
        }
    }

    /// Parses the JSON output of `xcodebuild -showBuildSettings -json`.
    pub fn from_show_build_settings_json(json_str: &str) -> Option<Self> {
        let parsed: Value = serde_json::from_str(json_str).ok()?;
        let entry = match &parsed {
            Value::Array(items) => items.first()?,
            Value::Object(_) => &parsed,
            _ => return None,
        };
        let settings = entry.get("buildSettings")?.as_object()?;
        let setting = |key: &str| {
            settings
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
        };

        let product_name = setting("PRODUCT_NAME")
            .or_else(|| setting("FULL_PRODUCT_NAME"))
            .or_else(|| setting("TARGET_NAME"))
            .or_else(|| {
                entry
                    .get("target")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
            })?
            .to_string();
        let executable_name = setting("EXECUTABLE_NAME")
            .unwrap_or(&product_name)
            .to_string();
        let marketing_version = setting("MARKETING_VERSION").unwrap_or("0.1.0").to_string();
        let build_version = setting("CURRENT_PROJECT_VERSION").map(ToString::to_string);
        let bundle_identifier = setting("PRODUCT_BUNDLE_IDENTIFIER").map(ToString::to_string);
        let built_products_dir = setting("BUILT_PRODUCTS_DIR").map(normalize_host_path);

        Some(Self {
            product_name,
            executable_name,
            marketing_version,
            build_version,
            bundle_identifier,
            built_products_dir,
        })
    }

    /// Fallback metadata derived from build arguments when `-showBuildSettings`
    /// is unavailable.
    pub fn fallback(config: &BuildConfig) -> Self {
        let from_project_stem = extract_optional_str(config, "project")
            .or_else(|| extract_optional_str(config, "workspace"))
            .and_then(|p| Path::new(p).file_stem())
            .and_then(OsStr::to_str)
            .filter(|s| !s.is_empty());

        let product_name = extract_optional_str(config, "product-name")
            .or_else(|| extract_optional_str(config, "scheme"))
            .or_else(|| extract_optional_str(config, "xcode-target"))
            .or(from_project_stem)
            .unwrap_or("App")
            .to_string();

        Self {
            executable_name: product_name.clone(),
            product_name,
            marketing_version: "0.1.0".to_string(),
            build_version: Some("1".to_string()),
            bundle_identifier: None,
            built_products_dir: None,
        }
    }
}

fn normalize_host_path(raw: &str) -> PathBuf {
    Path::new(raw).components().collect()
}

fn lookup_env(key: &str, environment: Option<&HashMap<String, String>>) -> Option<String> {
    environment
        .and_then(|env| env.get(key))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::env::var(key)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
}

fn effective_path(environment: Option<&HashMap<String, String>>) -> Option<std::ffi::OsString> {
    if let Some(env) = environment
        && let Some(path) = env.get("PATH")
    {
        return Some(std::ffi::OsString::from(path));
    }
    std::env::var_os("PATH")
}

fn which_in_path(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    let suffixes: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    std::env::split_paths(path_var).find_map(|dir| {
        suffixes.iter().find_map(|suffix| {
            let candidate = dir.join(format!("{name}{suffix}"));
            candidate.is_file().then_some(candidate)
        })
    })
}

fn is_crossswift_xcodebuild(path: &Path, environment: Option<&HashMap<String, String>>) -> bool {
    let mut cmd = Command::new(path);
    cmd.arg("-version");
    if let Some(env) = environment {
        cmd.envs(env);
    }
    let Ok(output) = cmd.output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.contains("CrossSwift")
}

/// Resolves how to invoke CrossXcodeBuild.
///
/// Resolution order:
/// 1. `xcodebuild-path` / `crossxcodebuild-path` build argument.
/// 2. `CROSSXCODEBUILD_PATH` / `CROSSXCODEBUILD` environment variable.
/// 3. `crossxcodebuild` executable on `PATH`.
/// 4. `xcodebuild` executable on `PATH` (directly on Windows/Linux; on macOS
///    only when `xcodebuild -version` reports `CrossSwift` so Apple's
///    `/usr/bin/xcodebuild` is never invoked by mistake).
/// 5. `swift run --package-path <crossswift-path> xcodebuild` using the
///    `crossswift-path` build argument or `CROSS_SWIFTUI_PATH` environment
///    variable.
pub fn resolve_cross_xcodebuild_command(
    config: &BuildConfig,
    environment: Option<&HashMap<String, String>>,
) -> Result<CrossXcodeBuildCommand, BuildError> {
    if let Some(explicit) = extract_optional_str(config, "xcodebuild-path")
        .or_else(|| extract_optional_str(config, "crossxcodebuild-path"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Ok(CrossXcodeBuildCommand {
            program: PathBuf::from(explicit),
            prefix_args: Vec::new(),
        });
    }

    if let Some(explicit) = lookup_env("CROSSXCODEBUILD_PATH", environment)
        .or_else(|| lookup_env("CROSSXCODEBUILD", environment))
    {
        return Ok(CrossXcodeBuildCommand {
            program: PathBuf::from(explicit),
            prefix_args: Vec::new(),
        });
    }

    let path_var = effective_path(environment);

    if let Some(path_var) = path_var.as_deref() {
        if let Some(found) = which_in_path("crossxcodebuild", path_var) {
            return Ok(CrossXcodeBuildCommand {
                program: found,
                prefix_args: Vec::new(),
            });
        }
        if let Some(found) = which_in_path("xcodebuild", path_var) {
            let is_macos = Platform::current() == Some(Platform::MacOS);
            if !is_macos || is_crossswift_xcodebuild(&found, environment) {
                return Ok(CrossXcodeBuildCommand {
                    program: found,
                    prefix_args: Vec::new(),
                });
            }
        }
    }

    if let Some(crossswift_dir) = extract_optional_str(config, "crossswift-path")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| lookup_env("CROSS_SWIFTUI_PATH", environment))
    {
        let swift = path_var
            .as_deref()
            .and_then(|pv| which_in_path("swift", pv))
            .unwrap_or_else(|| PathBuf::from("swift"));
        return Ok(CrossXcodeBuildCommand {
            program: swift,
            prefix_args: vec![
                "run".to_string(),
                "--package-path".to_string(),
                crossswift_dir,
                "xcodebuild".to_string(),
            ],
        });
    }

    Err(BuildError::Io(
        "Could not locate CrossXcodeBuild (`libnativeapi/crossswift`). \
         Install `crossxcodebuild` (or CrossSwift's `xcodebuild` on Windows/Linux) on PATH, \
         pass `xcodebuild-path` / `CROSSXCODEBUILD_PATH`, or set `crossswift-path` / `CROSS_SWIFTUI_PATH`."
            .to_string(),
    ))
}

fn parse_optional_bool(config: &BuildConfig, key: &str) -> Result<bool, BuildError> {
    match config.arguments.get(key) {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::Null) => Ok(true),
        Some(Value::String(s)) if s.eq_ignore_ascii_case("true") => Ok(true),
        Some(Value::String(s)) if s.eq_ignore_ascii_case("false") => Ok(false),
        Some(other) => Err(BuildError::InvalidArgument(format!(
            "`{key}` must be true or false, got {other}"
        ))),
    }
}

fn normalized_configuration(config: &BuildConfig) -> Result<&'static str, BuildError> {
    let raw = extract_optional_str(config, "configuration").unwrap_or("Release");
    if raw.eq_ignore_ascii_case("release") {
        Ok("Release")
    } else if raw.eq_ignore_ascii_case("debug") {
        Ok("Debug")
    } else {
        Err(BuildError::InvalidArgument(format!(
            "Unsupported CrossXcodeBuild configuration `{raw}` (expected `Debug` or `Release`)."
        )))
    }
}

fn validate_optional_nonempty_str<'a>(
    config: &'a BuildConfig,
    key: &str,
) -> Result<Option<&'a str>, BuildError> {
    match config.arguments.get(key) {
        None => Ok(None),
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(Some(s.as_str())),
        Some(other) => Err(BuildError::InvalidArgument(format!(
            "`{key}` must be a non-empty string, got {other}"
        ))),
    }
}

// ─── CrossXcodeBuilder ────────────────────────────────────────────────────────

/// An [`AppBuilder`] for building Xcode projects for non-Apple platforms via
/// CrossXcodeBuild (`libnativeapi/crossswift`).
///
/// Supported platforms: `windows`, `linux`, `web` (WebAssembly), `android`,
/// and `ohos` (HarmonyOS).
pub struct CrossXcodeBuilder {
    platform: Platform,
}

impl CrossXcodeBuilder {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    /// Common CLI flags shared between `-showBuildSettings` and `build`.
    pub fn common_arguments(&self, config: &BuildConfig) -> Result<Vec<String>, BuildError> {
        let mut args = Vec::new();

        if let Some(project) = validate_optional_nonempty_str(config, "project")? {
            args.extend(["-project".to_string(), project.to_string()]);
        } else if let Some(workspace) = validate_optional_nonempty_str(config, "workspace")? {
            args.extend(["-workspace".to_string(), workspace.to_string()]);
        }

        if let Some(scheme) = validate_optional_nonempty_str(config, "scheme")? {
            args.extend(["-scheme".to_string(), scheme.to_string()]);
        }

        if let Some(target) = validate_optional_nonempty_str(config, "xcode-target")? {
            args.extend(["-target".to_string(), target.to_string()]);
        }

        let configuration = normalized_configuration(config)?;
        args.extend(["-configuration".to_string(), configuration.to_string()]);

        if let Some(ddp) = validate_optional_nonempty_str(config, "derived-data-path")? {
            args.extend(["-derivedDataPath".to_string(), ddp.to_string()]);
        }

        let destination = validate_optional_nonempty_str(config, "destination")?;
        let sdk = validate_optional_nonempty_str(config, "sdk")?;
        if let Some(dest) = destination {
            args.extend(["-destination".to_string(), dest.to_string()]);
        }
        if let Some(sdk) = sdk {
            args.extend(["-sdk".to_string(), sdk.to_string()]);
        }
        if destination.is_none() && sdk.is_none() {
            let dest = default_destination(self.platform).ok_or_else(|| {
                BuildError::UnsupportedBuilder(format!(
                    "CrossXcodeBuilder does not support platform `{}`",
                    self.platform.as_str()
                ))
            })?;
            args.extend(["-destination".to_string(), dest.to_string()]);
        }

        if let Some(arch) = validate_optional_nonempty_str(config, "arch")? {
            args.extend(["-arch".to_string(), arch.to_string()]);
        }

        if let Some(xcconfig) = validate_optional_nonempty_str(config, "xcconfig-override")? {
            args.extend(["-xcconfig".to_string(), xcconfig.to_string()]);
        }

        if let Some(crossswift_path) = validate_optional_nonempty_str(config, "crossswift-path")? {
            args.extend(["--crossswift-path".to_string(), crossswift_path.to_string()]);
        }

        if let Some(nativeapi_path) = validate_optional_nonempty_str(config, "nativeapi-path")? {
            args.extend(["--nativeapi-path".to_string(), nativeapi_path.to_string()]);
        }

        if let Some(scratch_path) = validate_optional_nonempty_str(config, "scratch-path")? {
            args.extend(["--scratch-path".to_string(), scratch_path.to_string()]);
        }

        if parse_optional_bool(config, "console")? {
            args.push("--console".to_string());
        }

        if let Some(settings) = config.arguments.get("build-settings") {
            let Some(map) = settings.as_object() else {
                return Err(BuildError::InvalidArgument(
                    "`build-settings` must be an object of KEY to VALUE pairs".to_string(),
                ));
            };
            for (key, value) in map {
                let val = match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                args.push(format!("{key}={val}"));
            }
        }

        Ok(args)
    }

    /// Full argument list for running a CrossXcodeBuild build action.
    pub fn build_arguments(&self, config: &BuildConfig) -> Result<Vec<String>, BuildError> {
        let mut args = self.common_arguments(config)?;

        if let Some(flags) = config.arguments.get("extra-flags") {
            let Some(items) = flags.as_array() else {
                return Err(BuildError::InvalidArgument(
                    "`extra-flags` must be an array of strings".to_string(),
                ));
            };
            for item in items {
                let Some(flag) = item.as_str() else {
                    return Err(BuildError::InvalidArgument(format!(
                        "`extra-flags` items must be strings, got {item}"
                    )));
                };
                args.push(flag.to_string());
            }
        }

        if parse_optional_bool(config, "clean")? {
            args.push("clean".to_string());
        }
        args.push("build".to_string());

        Ok(args)
    }

    /// Default `BUILT_PRODUCTS_DIR` computed according to CrossXcodeBuild's
    /// layout: `<derivedDataPath>/Build/Products/<Configuration>` where
    /// `derivedDataPath` defaults to `<projectRoot>/build`.
    pub fn default_built_products_dir(&self, config: &BuildConfig) -> Result<PathBuf, BuildError> {
        let configuration = normalized_configuration(config)?;
        let derived_data = if let Some(ddp) = extract_optional_str(config, "derived-data-path") {
            normalize_host_path(ddp)
        } else {
            let project_root = extract_optional_str(config, "project")
                .or_else(|| extract_optional_str(config, "workspace"))
                .and_then(|p| Path::new(p).parent())
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.components().collect::<PathBuf>())
                .unwrap_or_else(|| PathBuf::from("."));
            project_root.join("build")
        };
        Ok(derived_data
            .join("Build")
            .join("Products")
            .join(configuration))
    }

    /// Resolves the staged product directory (`<BUILT_PRODUCTS_DIR>/<ProductName>`)
    /// and any platform-specific output files (`.apk` for Android, `.hap` for
    /// HarmonyOS).
    pub fn resolve_output_with_metadata(
        &self,
        config: &BuildConfig,
        metadata: Option<&CrossXcodeProjectMetadata>,
    ) -> Result<(PathBuf, Vec<PathBuf>), BuildError> {
        let products_dir = metadata
            .and_then(|m| m.built_products_dir.clone())
            .map(Ok)
            .unwrap_or_else(|| self.default_built_products_dir(config))?;

        let preferred_name = extract_optional_str(config, "product-name")
            .or_else(|| metadata.map(|m| m.product_name.as_str()));

        let staged_dir = resolve_staged_product_dir(&products_dir, preferred_name);

        let output_files = match self.platform {
            Platform::Windows | Platform::Linux | Platform::Web => Vec::new(),
            Platform::Android => find_artifacts(&staged_dir, "apk", &["app/build/outputs/apk"])?,
            Platform::Ohos => {
                find_artifacts(&staged_dir, "hap", &["entry/build/default/outputs/default"])?
            }
            Platform::MacOS | Platform::IOS => Vec::new(),
        };

        Ok((staged_dir, output_files))
    }
}

fn resolve_staged_product_dir(products_dir: &Path, preferred_name: Option<&str>) -> PathBuf {
    if let Some(name) = preferred_name {
        let candidate = products_dir.join(name);
        if candidate.is_dir() {
            return candidate;
        }
    }

    // Fallback: if there is a single subdirectory in `BUILT_PRODUCTS_DIR`, use it.
    if let Ok(entries) = std::fs::read_dir(products_dir) {
        let dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        if dirs.len() == 1 {
            return dirs[0].clone();
        }
    }

    if let Some(name) = preferred_name {
        products_dir.join(name)
    } else {
        products_dir.to_path_buf()
    }
}

fn find_artifacts(
    staged_dir: &Path,
    extension: &str,
    fallback_subdirs: &[&str],
) -> Result<Vec<PathBuf>, BuildError> {
    let mut files = collect_files_with_extension(staged_dir, extension, false);
    if !files.is_empty() {
        return Ok(files);
    }

    for subdir in fallback_subdirs {
        let nested_root = staged_dir.join(subdir);
        if nested_root.is_dir() {
            files.extend(collect_files_with_extension(&nested_root, extension, true));
        }
    }
    sort_newest_first(&mut files);
    files.dedup();
    Ok(files)
}

fn collect_files_with_extension(root: &Path, extension: &str, recursive: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_inner(root, extension, recursive, &mut files);
    sort_newest_first(&mut files);
    files
}

fn collect_files_inner(root: &Path, extension: &str, recursive: bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
            {
                out.push(path);
            }
        } else if recursive && path.is_dir() {
            collect_files_inner(&path, extension, true, out);
        }
    }
}

fn sort_newest_first(files: &mut [PathBuf]) {
    files.sort_by(|a, b| {
        let am = a.metadata().and_then(|m| m.modified()).ok();
        let bm = b.metadata().and_then(|m| m.modified()).ok();
        bm.cmp(&am).then_with(|| a.cmp(b))
    });
}

impl AppBuilder for CrossXcodeBuilder {
    fn name(&self) -> &str {
        builder_name(self.platform).unwrap_or("cross-xcode")
    }

    fn platform(&self) -> Platform {
        self.platform
    }

    fn matches(&self, platform: &Platform, target: Option<&str>) -> bool {
        *platform == self.platform && target == builder_name(self.platform)
    }

    fn is_supported_on_current_platform(&self) -> bool {
        is_host_supported(self.platform, Platform::current())
    }

    fn build_subcommand(&self) -> &str {
        "build"
    }

    fn validate_arguments(&self, config: &BuildConfig) -> Result<(), BuildError> {
        if !is_cross_xcode_platform(self.platform) {
            return Err(BuildError::UnsupportedBuilder(format!(
                "CrossXcodeBuilder does not support platform `{}`",
                self.platform.as_str()
            )));
        }
        if config.arguments.contains_key("project") && config.arguments.contains_key("workspace") {
            return Err(BuildError::InvalidArgument(
                "Specify either `project` or `workspace`, not both.".to_string(),
            ));
        }
        self.build_arguments(config)?;
        Ok(())
    }

    fn outputs_directory(&self) -> bool {
        matches!(
            self.platform,
            Platform::Windows | Platform::Linux | Platform::Web
        )
    }

    fn resolve_output_files(
        &self,
        config: &BuildConfig,
        _environment: Option<&HashMap<String, String>>,
    ) -> Result<(PathBuf, Vec<PathBuf>), BuildError> {
        self.resolve_output_with_metadata(config, None)
    }

    fn build_result(
        &self,
        config: BuildConfig,
        output_directory: PathBuf,
        output_files: Vec<PathBuf>,
        duration_ms: u128,
    ) -> BuildResult {
        BuildResult {
            config,
            output_directory,
            output_files,
            duration_ms,
            platform: self.platform,
            target: builder_name(self.platform).map(ToString::to_string),
        }
    }
}

// ─── CrossXcodeAppBuilder ─────────────────────────────────────────────────────

/// High-level runner for CrossXcodeBuild (`libnativeapi/crossswift`).
#[derive(Default)]
pub struct CrossXcodeAppBuilder;

impl CrossXcodeAppBuilder {
    /// Queries project metadata via `<crossxcodebuild> -showBuildSettings -json`.
    pub fn inspect_metadata(
        &self,
        platform: Platform,
        config: &BuildConfig,
        environment: Option<&HashMap<String, String>>,
    ) -> Option<CrossXcodeProjectMetadata> {
        let builder = CrossXcodeBuilder::new(platform);
        let tool = resolve_cross_xcodebuild_command(config, environment).ok()?;
        let common = builder.common_arguments(config).ok()?;

        let mut cmd = tool.to_command();
        cmd.args(["-showBuildSettings", "-json"]);
        cmd.args(&common);
        if let Some(env) = environment {
            cmd.envs(env);
        }

        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        CrossXcodeProjectMetadata::from_show_build_settings_json(&stdout)
    }

    /// Runs CrossXcodeBuild and returns both the [`BuildResult`] and resolved
    /// [`CrossXcodeProjectMetadata`] (used by Fastforge packagers for app name,
    /// binary name, and version).
    pub fn build_with_metadata(
        &self,
        platform: &str,
        _target: Option<&str>,
        arguments: serde_json::Map<String, serde_json::Value>,
        environment: Option<HashMap<String, String>>,
    ) -> Result<(BuildResult, CrossXcodeProjectMetadata), BuildError> {
        let parsed_platform = Platform::from_str(platform).map_err(|_| {
            BuildError::UnsupportedBuilder(format!("Unknown platform `{platform}`"))
        })?;
        if !is_cross_xcode_platform(parsed_platform) {
            return Err(BuildError::UnsupportedBuilder(format!(
                "CrossXcodeAppBuilder supports windows, linux, web, android, ohos; got `{platform}`"
            )));
        }

        let builder = CrossXcodeBuilder::new(parsed_platform);
        if !builder.is_supported_on_current_platform() {
            return Err(BuildError::UnsupportedPlatform(format!(
                "{} is not supported on the current platform",
                builder.name()
            )));
        }

        let config = BuildConfig::new(arguments);
        builder.validate_arguments(&config)?;

        let tool = resolve_cross_xcodebuild_command(&config, environment.as_ref())?;
        let build_args = builder.build_arguments(&config)?;

        // Query build settings before building so we know the staged product
        // name, executable name, version, and effective BUILT_PRODUCTS_DIR.
        let metadata = self
            .inspect_metadata(parsed_platform, &config, environment.as_ref())
            .unwrap_or_else(|| CrossXcodeProjectMetadata::fallback(&config));

        let mut cmd = tool.to_command();
        cmd.args(&build_args);

        eprintln!(
            "$ {} {}",
            cmd.get_program().to_string_lossy(),
            cmd.get_args()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );

        let start = std::time::Instant::now();
        if let Some(env) = &environment {
            cmd.envs(env);
        }
        run(&mut cmd)?;
        let duration = start.elapsed().as_millis();

        let (output_directory, output_files) =
            builder.resolve_output_with_metadata(&config, Some(&metadata))?;

        if builder.outputs_directory() {
            if !output_directory.is_dir() {
                return Err(BuildError::ArtifactNotFound(format!(
                    "CrossXcodeBuild output directory not found: '{}'",
                    output_directory.display()
                )));
            }
        } else if output_files.is_empty() {
            let expected = match parsed_platform {
                Platform::Android => ".apk",
                Platform::Ohos => ".hap",
                _ => "artifact",
            };
            return Err(BuildError::ArtifactNotFound(format!(
                "No {expected} found in '{}'. Check your CrossXcodeBuild SDK and toolchain setup.",
                output_directory.display()
            )));
        }

        let result = builder.build_result(config, output_directory, output_files, duration);
        Ok((result, metadata))
    }

    pub fn build(
        &self,
        platform: &str,
        target: Option<&str>,
        arguments: serde_json::Map<String, serde_json::Value>,
        environment: Option<HashMap<String, String>>,
    ) -> Result<BuildResult, BuildError> {
        self.build_with_metadata(platform, target, arguments, environment)
            .map(|(result, _)| result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};

    fn config(pairs: &[(&str, Value)]) -> BuildConfig {
        let mut map = Map::new();
        for (k, v) in pairs {
            map.insert((*k).to_string(), v.clone());
        }
        BuildConfig::new(map)
    }

    #[test]
    fn host_support_matrix_matches_crossswift() {
        assert!(is_host_supported(
            Platform::Windows,
            Some(Platform::Windows)
        ));
        assert!(!is_host_supported(Platform::Windows, Some(Platform::MacOS)));
        assert!(!is_host_supported(Platform::Windows, Some(Platform::Linux)));

        assert!(is_host_supported(Platform::Linux, Some(Platform::Linux)));
        assert!(!is_host_supported(Platform::Linux, Some(Platform::MacOS)));
        assert!(!is_host_supported(Platform::Linux, Some(Platform::Windows)));

        for target in [Platform::Web, Platform::Android, Platform::Ohos] {
            assert!(is_host_supported(target, Some(Platform::MacOS)));
            assert!(is_host_supported(target, Some(Platform::Linux)));
            assert!(!is_host_supported(target, Some(Platform::Windows)));
        }

        assert!(!is_host_supported(Platform::MacOS, Some(Platform::MacOS)));
        assert!(!is_host_supported(Platform::IOS, Some(Platform::MacOS)));
    }

    #[test]
    fn build_arguments_include_platform_destination_and_crossswift_flags() {
        let builder = CrossXcodeBuilder::new(Platform::Web);
        let cfg = config(&[
            ("project", json!("Examples/XcodeDemo/XcodeDemo.xcodeproj")),
            ("scheme", json!("XcodeDemo")),
            ("configuration", json!("release")),
            ("derived-data-path", json!("build")),
            ("crossswift-path", json!("/opt/crossswift")),
            ("nativeapi-path", json!("/opt/nativeapi/bindings/swift")),
            ("scratch-path", json!("/tmp/scratch")),
            ("xcconfig-override", json!("Release.xcconfig")),
            (
                "build-settings",
                json!({"CODE_SIGNING_ALLOWED": "NO", "SWIFT_WASM_SDK": "swift-6.4.2-RELEASE_wasm"}),
            ),
            ("extra-flags", json!(["-quiet"])),
            ("clean", json!(true)),
        ]);

        let args = builder.build_arguments(&cfg).unwrap();
        assert_eq!(
            args,
            vec![
                "-project",
                "Examples/XcodeDemo/XcodeDemo.xcodeproj",
                "-scheme",
                "XcodeDemo",
                "-configuration",
                "Release",
                "-derivedDataPath",
                "build",
                "-destination",
                "generic/platform=Wasm",
                "-xcconfig",
                "Release.xcconfig",
                "--crossswift-path",
                "/opt/crossswift",
                "--nativeapi-path",
                "/opt/nativeapi/bindings/swift",
                "--scratch-path",
                "/tmp/scratch",
                "CODE_SIGNING_ALLOWED=NO",
                "SWIFT_WASM_SDK=swift-6.4.2-RELEASE_wasm",
                "-quiet",
                "clean",
                "build",
            ]
        );
    }

    #[test]
    fn explicit_sdk_skips_default_destination() {
        let builder = CrossXcodeBuilder::new(Platform::Ohos);
        let cfg = config(&[
            ("workspace", json!("MyApp.xcworkspace")),
            ("xcode-target", json!("MyApp")),
            ("sdk", json!("ohos")),
            ("console", json!(true)),
        ]);

        let args = builder.build_arguments(&cfg).unwrap();
        assert_eq!(
            args,
            vec![
                "-workspace",
                "MyApp.xcworkspace",
                "-target",
                "MyApp",
                "-configuration",
                "Release",
                "-sdk",
                "ohos",
                "--console",
                "build",
            ]
        );
    }

    #[test]
    fn validation_rejects_conflicting_or_invalid_arguments() {
        let builder = CrossXcodeBuilder::new(Platform::Windows);

        assert!(
            builder
                .validate_arguments(&config(&[
                    ("project", json!("A.xcodeproj")),
                    ("workspace", json!("A.xcworkspace")),
                ]))
                .is_err()
        );

        assert!(
            builder
                .validate_arguments(&config(&[("configuration", json!("Profile"))]))
                .is_err()
        );

        assert!(
            builder
                .validate_arguments(&config(&[("build-settings", json!("not-an-object"))]))
                .is_err()
        );

        assert!(
            builder
                .validate_arguments(&config(&[("extra-flags", json!("not-an-array"))]))
                .is_err()
        );

        assert!(
            CrossXcodeBuilder::new(Platform::MacOS)
                .validate_arguments(&config(&[]))
                .is_err()
        );
    }

    #[test]
    fn parses_show_build_settings_json() {
        let raw = r#"[
            {
                "action": "build",
                "target": "XcodeDemo",
                "buildSettings": {
                    "BUILT_PRODUCTS_DIR": "/tmp/demo/build/Build/Products/Release",
                    "PRODUCT_NAME": "XcodeDemo",
                    "EXECUTABLE_NAME": "XcodeDemoBin",
                    "MARKETING_VERSION": "1.2.3",
                    "CURRENT_PROJECT_VERSION": "42",
                    "PRODUCT_BUNDLE_IDENTIFIER": "com.example.XcodeDemo"
                }
            }
        ]"#;

        let meta = CrossXcodeProjectMetadata::from_show_build_settings_json(raw).unwrap();
        assert_eq!(meta.product_name, "XcodeDemo");
        assert_eq!(meta.executable_name, "XcodeDemoBin");
        assert_eq!(meta.marketing_version, "1.2.3");
        assert_eq!(meta.build_version.as_deref(), Some("42"));
        assert_eq!(meta.app_version(), "1.2.3+42");
        assert_eq!(
            meta.bundle_identifier.as_deref(),
            Some("com.example.XcodeDemo")
        );
        assert_eq!(
            meta.built_products_dir.as_deref(),
            Some(Path::new("/tmp/demo/build/Build/Products/Release"))
        );

        #[cfg(windows)]
        {
            let win_raw = r#"[{"action":"build","target":"Demo","buildSettings":{"BUILT_PRODUCTS_DIR":"E:/Projects/demo/build/Build/Products/Release","PRODUCT_NAME":"Demo"}}]"#;
            let win_meta =
                CrossXcodeProjectMetadata::from_show_build_settings_json(win_raw).unwrap();
            let dir = win_meta.built_products_dir.unwrap();
            assert_eq!(
                dir.to_string_lossy(),
                r"E:\Projects\demo\build\Build\Products\Release"
            );
            assert_eq!(
                dir.join("Demo").to_string_lossy(),
                r"E:\Projects\demo\build\Build\Products\Release\Demo"
            );
        }
    }

    #[test]
    fn resolves_explicit_binary_and_crossswift_package_fallback() {
        let explicit = resolve_cross_xcodebuild_command(
            &config(&[("xcodebuild-path", json!("/custom/crossxcodebuild"))]),
            None,
        )
        .unwrap();
        assert_eq!(explicit.program, PathBuf::from("/custom/crossxcodebuild"));
        assert!(explicit.prefix_args.is_empty());

        let env = HashMap::from([("PATH".to_string(), "".to_string())]);
        let swift_run = resolve_cross_xcodebuild_command(
            &config(&[("crossswift-path", json!("/src/crossswift"))]),
            Some(&env),
        )
        .unwrap();
        assert_eq!(swift_run.program, PathBuf::from("swift"));
        assert_eq!(
            swift_run.prefix_args,
            vec!["run", "--package-path", "/src/crossswift", "xcodebuild"]
        );

        let bin_dir = tempfile::tempdir().unwrap();
        let bin_name = if cfg!(windows) {
            "crossxcodebuild.exe"
        } else {
            "crossxcodebuild"
        };
        let bin_path = bin_dir.path().join(bin_name);
        std::fs::write(&bin_path, b"").unwrap();
        let path_env = HashMap::from([(
            "PATH".to_string(),
            bin_dir.path().to_string_lossy().to_string(),
        )]);
        let from_path = resolve_cross_xcodebuild_command(&config(&[]), Some(&path_env)).unwrap();
        assert_eq!(from_path.program, bin_path);
        assert!(from_path.prefix_args.is_empty());
    }

    #[test]
    fn resolves_staged_outputs_for_directory_and_file_platforms() {
        let tmp = tempfile::tempdir().unwrap();
        let products = tmp.path().join("Build/Products/Release");
        let staged = products.join("DemoApp");
        std::fs::create_dir_all(&staged).unwrap();
        std::fs::write(staged.join("DemoApp.apk"), b"apk").unwrap();
        std::fs::write(staged.join("DemoApp.hap"), b"hap").unwrap();

        let meta = CrossXcodeProjectMetadata {
            product_name: "DemoApp".to_string(),
            executable_name: "DemoApp".to_string(),
            marketing_version: "1.0.0".to_string(),
            build_version: Some("1".to_string()),
            bundle_identifier: None,
            built_products_dir: Some(products),
        };
        let cfg = config(&[]);

        let (win_dir, win_files) = CrossXcodeBuilder::new(Platform::Windows)
            .resolve_output_with_metadata(&cfg, Some(&meta))
            .unwrap();
        assert_eq!(win_dir, staged);
        assert!(win_files.is_empty());

        let (android_dir, android_files) = CrossXcodeBuilder::new(Platform::Android)
            .resolve_output_with_metadata(&cfg, Some(&meta))
            .unwrap();
        assert_eq!(android_dir, staged);
        assert_eq!(android_files, vec![staged.join("DemoApp.apk")]);

        let (ohos_dir, ohos_files) = CrossXcodeBuilder::new(Platform::Ohos)
            .resolve_output_with_metadata(&cfg, Some(&meta))
            .unwrap();
        assert_eq!(ohos_dir, staged);
        assert_eq!(ohos_files, vec![staged.join("DemoApp.hap")]);
    }

    #[test]
    fn runs_crossxcodebuild_end_to_end_with_fake_cli() {
        let Some(host_platform) = Platform::current() else {
            return;
        };
        let target_platform = match host_platform {
            Platform::Windows => Platform::Windows,
            Platform::Linux => Platform::Linux,
            Platform::MacOS => Platform::Web,
            _ => return,
        };

        let tmp = tempfile::tempdir().unwrap();
        let products_dir = tmp.path().join("out/Build/Products/Release");
        let staged_dir = products_dir.join("FakeApp");
        std::fs::create_dir_all(&staged_dir).unwrap();

        #[cfg(windows)]
        let script_path = {
            let p = tmp.path().join("fake_crossxcodebuild.cmd");
            let json_products = products_dir.to_string_lossy().replace('\\', "\\\\");
            let script = format!(
                "@echo off\r\n\
                 if \"%~1\"==\"-showBuildSettings\" (\r\n\
                   echo [{{\"action\":\"build\",\"target\":\"FakeApp\",\"buildSettings\":{{\"BUILT_PRODUCTS_DIR\":\"{json_products}\",\"PRODUCT_NAME\":\"FakeApp\",\"EXECUTABLE_NAME\":\"FakeApp\",\"MARKETING_VERSION\":\"2.1.0\",\"CURRENT_PROJECT_VERSION\":\"9\"}}}}]\r\n\
                   exit /b 0\r\n\
                 )\r\n\
                 exit /b 0\r\n"
            );
            std::fs::write(&p, script).unwrap();
            p
        };

        #[cfg(not(windows))]
        let script_path = {
            use std::os::unix::fs::PermissionsExt;
            let p = tmp.path().join("fake_crossxcodebuild.sh");
            let json_products = products_dir.to_string_lossy();
            let script = format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = \"-showBuildSettings\" ]; then\n\
                   printf '%s\\n' '[{{\"action\":\"build\",\"target\":\"FakeApp\",\"buildSettings\":{{\"BUILT_PRODUCTS_DIR\":\"{json_products}\",\"PRODUCT_NAME\":\"FakeApp\",\"EXECUTABLE_NAME\":\"FakeApp\",\"MARKETING_VERSION\":\"2.1.0\",\"CURRENT_PROJECT_VERSION\":\"9\"}}}}]'\n\
                   exit 0\n\
                 fi\n\
                 exit 0\n"
            );
            std::fs::write(&p, script).unwrap();
            let mut perms = std::fs::metadata(&p).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&p, perms).unwrap();
            p
        };

        let mut args = Map::new();
        args.insert("project".to_string(), json!("FakeApp.xcodeproj"));
        args.insert(
            "xcodebuild-path".to_string(),
            json!(script_path.to_string_lossy().to_string()),
        );

        let (result, metadata) = CrossXcodeAppBuilder
            .build_with_metadata(target_platform.as_str(), None, args, None)
            .unwrap();

        assert_eq!(result.platform, target_platform);
        assert_eq!(result.output_directory, staged_dir);
        assert!(result.output_files.is_empty());
        assert_eq!(metadata.product_name, "FakeApp");
        assert_eq!(metadata.executable_name, "FakeApp");
        assert_eq!(metadata.app_version(), "2.1.0+9");
    }
}
