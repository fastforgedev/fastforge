use std::path::Path;
use std::process::Command;

use fastforge_core::{
    AppPackager, PackageConfig, PackageError, PackageResult, Platform, ProjectSettings,
    environment_variables,
};
use serde::Deserialize;

/// Runs a user-provided script to produce the package artifact, mirroring
/// Dart's `AppPackageMakerCustom`.
///
/// The script and output extension are read from
/// `<platform>/packaging/custom/make_config.yaml`:
///
/// ```yaml
/// script: ./scripts/package.sh
/// # Omit or leave empty to produce a directory artifact instead of a file.
/// output_extension: tar.gz
/// ```
///
/// The script receives fastforge's packaging variables (see
/// [`PackageConfig::package_variables`], including `.fastforge/config.yaml`
/// `env:` entries) plus `OUTPUT_ARTIFACT_PATH`; variables without a value
/// (`BUILD_NUMBER`, `FLAVOR`, `CHANNEL`, ...) are left unset.
#[derive(Debug)]
pub struct CustomPackager {
    platform: Platform,
    pub script: String,
    pub output_extension: String,
}

#[derive(Deserialize)]
struct CustomMakeConfig {
    script: String,
    #[serde(default)]
    output_extension: String,
}

impl CustomPackager {
    pub fn new(platform: Platform, script: String, output_extension: String) -> Self {
        Self {
            platform,
            script,
            output_extension,
        }
    }

    /// Load the packager configuration from
    /// `<platform>/packaging/custom/make_config.yaml`.
    pub fn load(platform: Platform) -> Result<Self, PackageError> {
        let path = format!("{}/packaging/custom/make_config.yaml", platform.as_str());
        Self::from_yaml_file(platform, Path::new(&path))
    }

    pub fn from_yaml_file(platform: Platform, path: &Path) -> Result<Self, PackageError> {
        if !path.exists() {
            return Err(PackageError::NotFound(format!(
                "Custom packager requires a config file at {}",
                path.display()
            )));
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            PackageError::General(format!("Failed to read {}: {}", path.display(), e))
        })?;
        let cfg: CustomMakeConfig = serde_yaml::from_str(&content).map_err(|e| {
            PackageError::General(format!("Failed to parse {}: {}", path.display(), e))
        })?;
        Ok(Self::new(platform, cfg.script, cfg.output_extension))
    }
}

impl AppPackager for CustomPackager {
    fn name(&self) -> &str {
        "custom"
    }

    fn platform(&self) -> Platform {
        self.platform
    }

    fn package_format(&self) -> &str {
        &self.output_extension
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        // The artifact path uses the configured output extension instead of
        // the `custom` placeholder format.
        let mut effective = config.clone();
        effective.package_format = self.output_extension.clone();
        let output_path = effective.output_file();

        let build_name = effective
            .app_version
            .split('+')
            .next()
            .unwrap_or(&effective.app_version)
            .to_string();
        let build_number = effective.app_version.split('+').nth(1).map(str::to_string);

        let (shell, flag) = if cfg!(target_os = "windows") {
            ("cmd", "/c")
        } else {
            ("sh", "-c")
        };

        let settings = ProjectSettings::load()?;
        let variables = config.package_variables(&settings);

        let mut cmd = Command::new(shell);
        cmd.args([flag, &self.script]);
        cmd.envs(environment_variables(&variables));
        cmd.env("APP_NAME", &effective.app_name)
            .env("APP_VERSION", &effective.app_version)
            .env("BUILD_NAME", build_name)
            .env("BUILD_MODE", &effective.build_mode)
            .env("BUILD_OUTPUT_DIRECTORY", &effective.build_output_dir)
            .env("OUTPUT_DIRECTORY", &effective.output_dir)
            .env("OUTPUT_ARTIFACT_PATH", &output_path);
        if let Some(number) = build_number {
            cmd.env("BUILD_NUMBER", number);
        }
        if let Some(flavor) = &effective.flavor {
            cmd.env("FLAVOR", flavor);
        }
        if let Some(channel) = &effective.channel {
            cmd.env("CHANNEL", channel);
        }

        let out = cmd
            .output()
            .map_err(|e| PackageError::MissingTool(format!("{}: {}", shell, e)))?;
        if !out.status.success() {
            return Err(PackageError::CommandFailed {
                command: self.script.clone(),
                stderr: String::from_utf8_lossy(&out.stderr).into(),
            });
        }

        // Mirrors Dart's `DefaultMakeResultResolver`: the script must have
        // produced the artifact (a directory when `output_extension` is empty).
        effective.resolve_result(output_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::collections::HashMap;

    fn write_config(dir: &Path, platform: &str, yaml: &str) -> std::path::PathBuf {
        let cfg_dir = dir.join(platform).join("packaging").join("custom");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        let path = cfg_dir.join("make_config.yaml");
        std::fs::write(&path, yaml).unwrap();
        path
    }

    #[test]
    fn loads_script_and_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_config(
            dir.path(),
            "linux",
            "script: ./scripts/package.sh\noutput_extension: tar.gz\n",
        );
        let packager = CustomPackager::from_yaml_file(Platform::Linux, &path).unwrap();
        assert_eq!(packager.script, "./scripts/package.sh");
        assert_eq!(packager.output_extension, "tar.gz");
        assert_eq!(packager.package_format(), "tar.gz");
    }

    #[test]
    fn missing_config_is_an_error() {
        let err = CustomPackager::from_yaml_file(
            Platform::Linux,
            Path::new("/nonexistent/make_config.yaml"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("make_config.yaml"));
    }

    #[cfg(unix)]
    #[test]
    fn missing_artifact_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "demo".into(),
            app_version: "1.0.0".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "custom".into(),
            is_installer: false,
            build_output_dir: dir.path().to_path_buf(),
            build_output_files: vec![],
            output_dir: dir.path().join("dist"),
            environment: Default::default(),
        };
        let file = CustomPackager::new(Platform::Linux, "true".into(), "tar.gz".into());
        assert_eq!(
            file.package(&config).unwrap_err().to_string(),
            "No output file found."
        );
        let directory = CustomPackager::new(Platform::Linux, "true".into(), String::new());
        assert_eq!(
            directory.package(&config).unwrap_err().to_string(),
            "No output directory found."
        );
    }

    #[cfg(unix)]
    #[test]
    fn runs_script_with_environment() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = dir.path().join("dist");
        let capture = dir.path().join("env.txt");
        let script = format!(
            "echo \"$APP_NAME|$APP_VERSION|$BUILD_NAME|$BUILD_NUMBER|$BUILD_MODE|$FLAVOR|$CHANNEL\" > {} && touch \"$OUTPUT_ARTIFACT_PATH\"",
            capture.display()
        );
        let packager = CustomPackager::new(Platform::Linux, script, "tar.gz".to_string());

        let config = PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "demo".into(),
            app_version: "1.0.0+7".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: Some("dev".into()),
            channel: Some("beta".into()),
            artifact_name: None,
            package_format: "custom".into(),
            is_installer: false,
            build_output_dir: dir.path().to_path_buf(),
            build_output_files: vec![],
            output_dir: out_dir.clone(),
            environment: Default::default(),
        };

        let result = packager.package(&config).unwrap();
        let artifact = &result.artifacts[0];
        assert!(artifact.exists(), "script should create the artifact");
        assert!(
            artifact.to_string_lossy().ends_with(".tar.gz"),
            "artifact should use the configured extension: {}",
            artifact.display()
        );

        let captured = std::fs::read_to_string(&capture).unwrap();
        let parts: HashMap<usize, &str> = captured.trim().split('|').enumerate().collect();
        assert_eq!(parts[&0], "demo");
        assert_eq!(parts[&1], "1.0.0+7");
        assert_eq!(parts[&2], "1.0.0");
        assert_eq!(parts[&3], "7");
        assert_eq!(parts[&4], "release");
        assert_eq!(parts[&5], "dev");
        assert_eq!(parts[&6], "beta");
    }
}
