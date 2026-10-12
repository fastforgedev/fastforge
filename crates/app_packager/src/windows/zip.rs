use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

use crate::fs_util::zip_dir_contents_excluding;

use super::common::is_debug_symbols;

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
        zip_dir_contents_excluding(&config.build_output_dir, &output_file, &is_debug_symbols)?;
        config.resolve_result(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows::common::tests::{config, write};
    use crate::windows::direct::WindowsDirectPackager;

    #[test]
    fn zip_and_direct_leave_out_debug_symbols() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_config = config(tmp.path(), "zip");
        let build = &zip_config.build_output_dir;
        write(&build.join("runner.exe"), "exe");
        write(&build.join("runner.pdb"), "pdb");
        write(&build.join("plugin/Helper.PDB"), "pdb");
        write(&build.join("data/app.so"), "so");

        let zip = WindowsZipPackager.package(&zip_config).unwrap();
        let archive =
            zip::ZipArchive::new(std::fs::File::open(&zip.artifacts[0]).unwrap()).unwrap();
        let mut names: Vec<_> = archive.file_names().map(String::from).collect();
        names.sort();
        assert_eq!(names, ["data/", "data/app.so", "plugin/", "runner.exe"]);

        let direct = WindowsDirectPackager
            .package(&config(tmp.path(), ""))
            .unwrap();
        assert!(direct.artifacts[0].join("runner.exe").is_file());
        assert!(!direct.artifacts[0].join("runner.pdb").exists());
        assert!(!direct.artifacts[0].join("plugin/Helper.PDB").exists());
    }
}
