use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

use crate::fs_util::{copy_dir_contents, zip_dir_contents};

/// Copies the first build output file (the `.ipa` exported by Xcode)
/// to the versioned output path, mirroring Dart's `AppPackageMakerIpa`.
///
/// An unsigned `.app` (from `--build-no-codesign`) is wrapped into an IPA
/// as `Payload/<name>.app`, for signing later or for tools that take one.
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
        if src.is_dir() {
            let app_name = src
                .file_name()
                .ok_or_else(|| PackageError::General("invalid app bundle path".into()))?;
            let staging = config.packaging_dir()?;
            copy_dir_contents(src, &staging.join("Payload").join(app_name))?;
            let zipped = zip_dir_contents(&staging, &dst);
            std::fs::remove_dir_all(&staging).ok();
            zipped?;
        } else {
            std::fs::copy(src, &dst)?;
        }
        config.resolve_result(dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn config(tmp: &std::path::Path, src: std::path::PathBuf) -> PackageConfig {
        PackageConfig {
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
            build_output_dir: tmp.to_path_buf(),
            build_output_files: vec![src],
            output_dir: tmp.to_path_buf(),
            environment: Default::default(),
        }
    }

    #[test]
    fn copies_ipa_to_output() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("Runner.ipa");
        std::fs::write(&src, b"fake-ipa").unwrap();

        let result = IOSIpaPackager.package(&config(tmp.path(), src)).unwrap();
        assert!(result.artifacts[0].exists());
    }

    #[test]
    fn wraps_unsigned_app_in_payload() {
        let tmp = TempDir::new().unwrap();
        let app = tmp
            .path()
            .join("Runner.xcarchive/Products/Applications/Runner.app");
        std::fs::create_dir_all(app.join("Frameworks")).unwrap();
        std::fs::write(app.join("Runner"), b"binary").unwrap();
        std::fs::write(app.join("Frameworks/App"), b"dart").unwrap();

        let result = IOSIpaPackager.package(&config(tmp.path(), app)).unwrap();
        let ipa = &result.artifacts[0];
        assert_eq!(ipa.extension().unwrap(), "ipa");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(ipa).unwrap()).unwrap();
        let mut names: Vec<String> = archive.file_names().map(str::to_string).collect();
        names.sort();
        assert_eq!(
            names,
            [
                "Payload/",
                "Payload/Runner.app/",
                "Payload/Runner.app/Frameworks/",
                "Payload/Runner.app/Frameworks/App",
                "Payload/Runner.app/Runner",
            ]
        );
        assert!(archive.by_name("Payload/Runner.app/Runner").is_ok());
        // The staging directory is cleaned up next to the artifact.
        let leftovers: Vec<_> = std::fs::read_dir(ipa.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(leftovers.len(), 1);
    }
}
