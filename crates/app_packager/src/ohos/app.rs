use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

/// Copies the first build output file (the `.app` produced by the HarmonyOS SDK)
/// to the versioned output path, mirroring Dart's `AppPackageMakerApp`.
pub struct OHOSAppPackager;

impl AppPackager for OHOSAppPackager {
    fn name(&self) -> &str {
        "app"
    }

    fn platform(&self) -> Platform {
        Platform::Ohos
    }

    fn package_format(&self) -> &str {
        "app"
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
    fn copies_app_to_output() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("app.app");
        std::fs::write(&src, b"fake-app").unwrap();

        let cfg = PackageConfig {
            app_name: "myapp".into(),
            app_binary_name: "myapp".into(),
            app_version: "1.0.0".into(),
            build_mode: "release".into(),
            platform: Platform::Ohos,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "app".into(),
            is_installer: false,
            build_output_dir: tmp.path().to_path_buf(),
            build_output_files: vec![src],
            output_dir: tmp.path().to_path_buf(),
            environment: Default::default(),
        };
        let result = OHOSAppPackager.package(&cfg).unwrap();
        assert!(result.artifacts[0].exists());
    }
}
