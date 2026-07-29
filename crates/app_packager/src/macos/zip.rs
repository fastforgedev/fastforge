use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

/// Zips a macOS `.app` bundle using `7z`, mirroring the macOS branch of
/// Dart's `AppPackageMakerZip`.
///
/// The Dart implementation notes that `archive` (pure-Dart zip) corrupts `.app`
/// bundles, so `7z` is used instead; the same approach is applied here.
pub struct MacOSZipPackager;

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

    #[cfg(not(target_os = "macos"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let pkg_dir = config.packaging_dir();

        // Find the .app bundle
        let app_bundle = std::fs::read_dir(&config.build_output_dir)?
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().is_some_and(|x| x == "app"))
            .ok_or_else(|| PackageError::NotFound(".app bundle in build output".into()))?;

        // Copy .app into packaging directory
        run(Command::new("cp").args([
            "-RH",
            &app_bundle.path().display().to_string(),
            &pkg_dir.display().to_string(),
        ]))?;

        let output_file = config.output_file();
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
        Ok(PackageResult {
            artifacts: vec![output_file],
        })
    }
}
