use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

/// Copies the first build output file (the `.aab` produced by `flutter build appbundle`)
/// to the versioned output path, mirroring Dart's `AppPackageMakerAab`.
pub struct AndroidAabPackager;

impl AppPackager for AndroidAabPackager {
    fn name(&self) -> &str {
        "aab"
    }

    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn package_format(&self) -> &str {
        "aab"
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
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_config(tmp: &TempDir, src_file: &std::path::Path) -> PackageConfig {
        PackageConfig {
            app_name: "myapp".into(),
            app_binary_name: "myapp".into(),
            app_version: "1.0.0".into(),
            build_mode: "release".into(),
            platform: Platform::Android,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "aab".into(),
            is_installer: false,
            build_output_dir: tmp.path().to_path_buf(),
            build_output_files: vec![src_file.to_path_buf()],
            output_dir: tmp.path().to_path_buf(),
            environment: Default::default(),
        }
    }

    #[test]
    fn copies_aab_to_output() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("app-release.aab");
        std::fs::write(&src, b"fake-aab").unwrap();

        let cfg = make_config(&tmp, &src);
        let result = AndroidAabPackager.package(&cfg).unwrap();
        assert_eq!(result.artifacts.len(), 1);
        assert!(result.artifacts[0].exists());
    }

    #[test]
    fn errors_when_no_build_output_files() {
        let tmp = TempDir::new().unwrap();
        let mut cfg = make_config(&tmp, &PathBuf::from("nonexistent.aab"));
        cfg.build_output_files.clear();
        assert!(AndroidAabPackager.package(&cfg).is_err());
    }
}
