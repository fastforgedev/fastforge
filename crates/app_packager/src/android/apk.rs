use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

/// Copies the first build output file (the `.apk` produced by `flutter build apk`)
/// to the versioned output path, mirroring Dart's `AppPackageMakerApk`.
pub struct AndroidApkPackager;

impl AppPackager for AndroidApkPackager {
    fn name(&self) -> &str {
        "apk"
    }

    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn package_format(&self) -> &str {
        "apk"
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let src = config
            .first_build_output_file()
            .ok_or_else(|| PackageError::General("no build output files".into()))?;
        let dst = config.output_file()?;
        std::fs::copy(src, &dst)?;
        config.resolve_result(dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn copies_apk_to_output() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("app-release.apk");
        std::fs::write(&src, b"fake-apk").unwrap();

        let cfg = PackageConfig {
            app_name: "myapp".into(),
            app_binary_name: "myapp".into(),
            app_version: "1.0.0".into(),
            build_mode: "release".into(),
            platform: Platform::Android,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "apk".into(),
            is_installer: false,
            build_output_dir: tmp.path().to_path_buf(),
            build_output_files: vec![src],
            output_dir: tmp.path().to_path_buf(),
            environment: Default::default(),
        };
        let result = AndroidApkPackager.package(&cfg).unwrap();
        assert!(result.artifacts[0].exists());
    }
}
