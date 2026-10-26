use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

use crate::fs_util::copy_dir_contents;

/// Copies the flutter Linux build output directory to the output path
/// without any additional packaging, mirroring Dart's
/// `AppPackageMakerDirect("linux")`. Like Dart's `copyPathSync`, the build
/// output *contents* are merged into the destination directory, so re-running
/// never nests the build directory inside an existing output.
pub struct LinuxDirectPackager;

impl AppPackager for LinuxDirectPackager {
    fn name(&self) -> &str {
        "direct"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        ""
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        // The destination directory uses the rendered artifact name (no
        // extension for direct output).
        let dst = config.output_file()?;
        copy_dir_contents(&config.build_output_dir, &dst)?;
        config.resolve_result(dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rerun_does_not_nest_build_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("bundle");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("demo"), b"bin").unwrap();
        let config = PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "demo".into(),
            app_version: "1.0.0".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: String::new(),
            is_installer: false,
            build_output_dir: bundle,
            build_output_files: vec![],
            output_dir: tmp.path().join("dist"),
            packaging_root: None,
            environment: Default::default(),
        };
        let first = LinuxDirectPackager.package(&config).unwrap();
        let second = LinuxDirectPackager.package(&config).unwrap();
        assert_eq!(first.artifacts, second.artifacts);
        let dir = &second.artifacts[0];
        assert!(dir.join("demo").is_file());
        assert!(!dir.join("bundle").exists());
    }
}
