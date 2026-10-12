use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

use crate::fs_util::copy_dir_contents_excluding;

use super::common::is_debug_symbols;

/// Copies the flutter Windows build output directory to the output path
/// without any additional packaging, mirroring Dart's
/// `AppPackageMakerDirect("windows")`. Like Dart's `copyPathSync`, the build
/// output *contents* are merged into the destination directory, so re-running
/// never nests the build directory inside an existing output.
pub struct WindowsDirectPackager;

impl AppPackager for WindowsDirectPackager {
    fn name(&self) -> &str {
        "direct"
    }

    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn package_format(&self) -> &str {
        ""
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        // The destination directory uses the rendered artifact name (no
        // extension for direct output).
        let dst = config.output_file()?;
        copy_dir_contents_excluding(&config.build_output_dir, &dst, &is_debug_symbols)?;
        config.resolve_result(dst)
    }
}
