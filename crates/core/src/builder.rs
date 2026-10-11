use crate::model::Platform;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildMode {
    Profile,
    Release,
}

impl BuildMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Release => "release",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildConfig {
    pub arguments: Map<String, Value>,
}

impl BuildConfig {
    pub fn new(arguments: Map<String, Value>) -> Self {
        Self { arguments }
    }

    pub fn mode(&self) -> BuildMode {
        if self.arguments.contains_key("profile") {
            BuildMode::Profile
        } else {
            BuildMode::Release
        }
    }

    pub fn flavor(&self) -> Option<&str> {
        self.arguments.get("flavor").and_then(Value::as_str)
    }

    pub fn to_json_compatible(&self) -> Value {
        let mut obj = Map::new();
        obj.insert(
            "mode".to_string(),
            Value::String(self.mode().as_str().to_string()),
        );
        if let Some(flavor) = self.flavor() {
            obj.insert("flavor".to_string(), Value::String(flavor.to_string()));
        }
        obj.insert(
            "arguments".to_string(),
            Value::Object(self.arguments.clone()),
        );
        Value::Object(obj)
    }
}

/// ABIs that Android splits APKs by, as named in split APK files.
const ANDROID_APK_ABIS: [&str; 4] = ["armeabi-v7a", "arm64-v8a", "x86_64", "x86"];

/// The ABI of a split APK, from its file name: Flutter writes
/// `app-<abi>[-<flavor>]-<mode>.apk` for `--split-per-abi`, and Gradle ABI
/// splits also name the ABI as a `-`-delimited segment. `None` for a
/// universal APK.
pub fn android_apk_abi(path: &Path) -> Option<&'static str> {
    let stem = path.file_stem()?.to_str()?;
    let segments = format!("-{}-", stem);
    ANDROID_APK_ABIS
        .into_iter()
        .find(|abi| segments.contains(&format!("-{}-", abi)))
}

#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub platform: Platform,
    pub target: Option<String>,
    pub arguments: Map<String, Value>,
    pub environment: Option<HashMap<String, String>>,
}

impl BuildRequest {
    pub fn config(&self) -> BuildConfig {
        BuildConfig::new(self.arguments.clone())
    }
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub config: BuildConfig,
    pub output_directory: PathBuf,
    pub output_files: Vec<PathBuf>,
    pub duration_ms: u128,
    pub platform: Platform,
    pub target: Option<String>,
}

impl BuildResult {
    pub fn to_json_compatible(&self) -> Value {
        json!({
            "config": self.config.to_json_compatible(),
            "outputDirectory": self.output_directory.to_string_lossy().to_string(),
            "duration": self.duration_ms,
            "outputFiles": self.output_files.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, Error)]
pub enum BuildError {
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("Unsupported builder: {0}")]
    UnsupportedBuilder(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Artifact not found: {0}")]
    ArtifactNotFound(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait AppBuilder {
    fn name(&self) -> &str;
    fn platform(&self) -> Platform;
    fn matches(&self, platform: &Platform, target: Option<&str>) -> bool;
    fn is_supported_on_current_platform(&self) -> bool;
    fn build_subcommand(&self) -> &str;
    fn validate_arguments(&self, _config: &BuildConfig) -> Result<(), BuildError> {
        Ok(())
    }
    /// Whether the build output is a whole directory (linux/windows/web
    /// bundles). Such builders report no individual `output_files`, as in
    /// Dart, and only the output directory has to exist.
    fn outputs_directory(&self) -> bool {
        false
    }
    fn resolve_output_files(
        &self,
        config: &BuildConfig,
        environment: Option<&HashMap<String, String>>,
    ) -> Result<(PathBuf, Vec<PathBuf>), BuildError>;
    fn build_result(
        &self,
        config: BuildConfig,
        output_directory: PathBuf,
        output_files: Vec<PathBuf>,
        duration_ms: u128,
    ) -> BuildResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_apk_abi_from_file_name() {
        let abi = |name: &str| android_apk_abi(Path::new(name));
        assert_eq!(abi("app-arm64-v8a-release.apk"), Some("arm64-v8a"));
        assert_eq!(abi("app-armeabi-v7a-dev-release.apk"), Some("armeabi-v7a"));
        assert_eq!(abi("app-x86_64-profile.apk"), Some("x86_64"));
        assert_eq!(abi("app-dev-x86-release.apk"), Some("x86"));
        assert_eq!(abi("app-release.apk"), None);
        assert_eq!(abi("app-dev-release.apk"), None);
        assert_eq!(abi("app-universal-release.apk"), None);
    }
}
