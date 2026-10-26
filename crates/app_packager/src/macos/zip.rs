use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

use super::notarize;

/// Zips a macOS `.app` bundle using `7z`, mirroring the macOS branch of
/// Dart's `AppPackageMakerZip`.
///
/// The Dart implementation notes that `archive` (pure-Dart zip) corrupts `.app`
/// bundles, so `7z` is used instead; the same approach is applied here.
/// Like Dart, the packager is not restricted to macOS hosts (it only needs
/// `cp` and `7z`).
pub struct MacOSZipPackager;

/// `<packaging root>/zip/make_config.yaml` (`macos/packaging/zip/make_config.yaml`
/// for Flutter projects; see `PackageConfig::packaging_root`).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ZipMakeConfig {
    /// Notarize and staple the `.app` before zipping it.
    #[serde(default)]
    notarize: bool,
}

impl ZipMakeConfig {
    fn load(path: &Path) -> Result<Self, PackageError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            PackageError::General(format!("Failed to read {}: {}", path.display(), e))
        })?;
        serde_yaml::from_str::<Option<Self>>(&content)
            .map(Option::unwrap_or_default)
            .map_err(|e| {
                PackageError::General(format!("Failed to parse {}: {}", path.display(), e))
            })
    }
}

fn run(cmd: &mut Command) -> Result<(), PackageError> {
    let out = cmd.output().map_err(|e| {
        PackageError::MissingTool(format!("{}: {}", cmd.get_program().to_string_lossy(), e))
    })?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: cmd.get_program().to_string_lossy().into(),
            stderr: String::from_utf8_lossy(&out.stderr).into(),
        });
    }
    Ok(())
}

impl AppPackager for MacOSZipPackager {
    fn name(&self) -> &str {
        "zip"
    }

    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn package_format(&self) -> &str {
        "zip"
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = ZipMakeConfig::load(&config.make_config_path("zip"))?;
        let credentials = notarize::enabled(make_config.notarize, config)
            .then(|| notarize::Credentials::from_config(config))
            .transpose()?;
        let pkg_dir = config.packaging_dir()?;

        // Find the .app bundle
        let app_bundle = std::fs::read_dir(&config.build_output_dir)?
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().is_some_and(|x| x == "app"))
            .ok_or_else(|| PackageError::NotFound(".app bundle in build output".into()))?;

        // A zip cannot be stapled, so the app is stapled before it is zipped.
        if let Some(credentials) = &credentials {
            notarize::notarize_app(&app_bundle.path(), credentials, &pkg_dir)?;
        }

        // Copy .app into packaging directory
        run(Command::new("cp").args([
            "-RH",
            &app_bundle.path().display().to_string(),
            &pkg_dir.display().to_string(),
        ]))?;

        let output_file = config.output_file()?;
        // 7z runs with its cwd set to pkg_dir, so a relative output_file would
        // resolve inside pkg_dir (and then get deleted by the cleanup below)
        // instead of landing at the intended destination.
        let absolute_output_file = if output_file.is_absolute() {
            output_file.clone()
        } else {
            std::env::current_dir()?.join(&output_file)
        };
        // 7z a <output.zip> *.app  (run from inside pkg_dir)
        run(Command::new("7z").current_dir(&pkg_dir).args([
            "a",
            &absolute_output_file.display().to_string(),
            "*.app",
        ]))?;

        std::fs::remove_dir_all(&pkg_dir).ok();
        config.resolve_result(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_config_reads_notarize() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("make_config.yaml");
        assert!(!ZipMakeConfig::load(&path).unwrap().notarize);
        std::fs::write(&path, "notarize: true\n").unwrap();
        assert!(ZipMakeConfig::load(&path).unwrap().notarize);
        std::fs::write(&path, "").unwrap();
        assert!(!ZipMakeConfig::load(&path).unwrap().notarize);
        std::fs::write(&path, "notarise: true\n").unwrap();
        assert!(ZipMakeConfig::load(&path).is_err());
    }
}
