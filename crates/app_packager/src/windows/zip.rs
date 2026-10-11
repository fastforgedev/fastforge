use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

use crate::fs_util::zip_dir_contents;

/// Zips a Windows flutter build output directory in-process, mirroring the
/// non-macOS branch of Dart's `AppPackageMakerZip` (`ZipFileEncoder
/// .zipDirectory`): the directory *contents* are archived without a
/// top-level folder, symlinks are followed, Unix permissions are kept and an
/// existing archive is overwritten.
pub struct WindowsZipPackager;

impl AppPackager for WindowsZipPackager {
    fn name(&self) -> &str {
        "zip"
    }

    fn platform(&self) -> Platform {
        Platform::Windows
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
