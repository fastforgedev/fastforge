use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

/// Copies the first build output file (the `.ipa` exported by Xcode)
/// to the versioned output path, mirroring Dart's `AppPackageMakerIpa`.
pub struct IOSIpaPackager;

impl AppPackager for IOSIpaPackager {
    fn name(&self) -> &str {
        "ipa"
    }

    fn platform(&self) -> Platform {
        Platform::IOS
    }

    fn package_format(&self) -> &str {
        "ipa"
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
    fn copies_ipa_to_output() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("Runner.ipa");
        std::fs::write(&src, b"fake-ipa").unwrap();

        let cfg = PackageConfig {
            app_name: "myapp".into(),
            app_binary_name: "myapp".into(),
            app_version: "1.0.0".into(),
            build_mode: "release".into(),
            platform: Platform::IOS,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "ipa".into(),
            is_installer: false,
            build_output_dir: tmp.path().to_path_buf(),
            build_output_files: vec![src],
            output_dir: tmp.path().to_path_buf(),
            environment: Default::default(),
        };
        let result = IOSIpaPackager.package(&cfg).unwrap();
        assert!(result.artifacts[0].exists());
    }
}
