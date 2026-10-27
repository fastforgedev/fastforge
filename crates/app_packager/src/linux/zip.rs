use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

use crate::fs_util::zip_dir_contents;

/// Zips a Linux flutter build output directory in-process, mirroring the
/// non-macOS branch of Dart's `AppPackageMakerZip` (`ZipFileEncoder
/// .zipDirectory`): the directory *contents* are archived without a
/// top-level folder, symlinks are followed, Unix permissions are kept and an
/// existing archive is overwritten.
pub struct LinuxZipPackager;

impl AppPackager for LinuxZipPackager {
    fn name(&self) -> &str {
        "zip"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        "zip"
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let output_file = config.output_file()?;
        zip_dir_contents(&config.build_output_dir, &output_file)?;
        config.resolve_result(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn zips_build_output_into_relative_output_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("build/linux/x64/release/bundle");
        std::fs::create_dir_all(bundle.join("lib")).unwrap();
        std::fs::write(bundle.join("demo"), b"bin").unwrap();
        std::fs::write(bundle.join("lib/libapp.so"), b"so").unwrap();

        let config = PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "demo".into(),
            app_version: "1.0.0+1".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "zip".into(),
            is_installer: false,
            build_output_dir: bundle,
            build_output_files: vec![],
            output_dir: tmp.path().join("dist"),
            packaging_root: None,
            environment: Default::default(),
        };
        let result = LinuxZipPackager.package(&config).unwrap();
        let artifact: &PathBuf = &result.artifacts[0];
        assert!(artifact.ends_with("dist/1.0.0+1/demo-1.0.0+1-linux.zip"));
        let archive = zip::ZipArchive::new(std::fs::File::open(artifact).unwrap()).unwrap();
        let names: Vec<&str> = archive.file_names().collect();
        assert!(names.contains(&"demo"));
        assert!(names.contains(&"lib/libapp.so"));
        // Re-running overwrites the existing archive.
        assert!(LinuxZipPackager.package(&config).is_ok());
    }
}
