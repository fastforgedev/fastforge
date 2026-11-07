use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use flate2::{Compression, write::GzEncoder};

/// A portable archive that preserves executable permissions and symbolic links.
pub struct TarGzPackager(pub Platform);

impl AppPackager for TarGzPackager {
    fn name(&self) -> &str {
        "tar.gz"
    }

    fn platform(&self) -> Platform {
        self.0
    }

    fn package_format(&self) -> &str {
        "tar.gz"
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let src = &config.build_output_dir;
        if !src.is_dir() {
            return Err(PackageError::NotFound(format!(
                "Directory not found: {}",
                src.display()
            )));
        }
        let mut roots = std::fs::read_dir(src)?.collect::<Result<Vec<_>, _>>()?;
        roots.sort_by_key(|entry| entry.file_name());
        if self.0 == Platform::MacOS {
            roots.retain(|entry| entry.path().extension().is_some_and(|ext| ext == "app"));
            if roots.is_empty() {
                return Err(PackageError::NotFound(".app bundle in build output".into()));
            }
            if crate::macos::notarize::enabled(false, config) {
                let credentials = crate::macos::notarize::Credentials::from_config(config)?;
                let staging = config.packaging_dir()?;
                for entry in &roots {
                    crate::macos::notarize::notarize_app(&entry.path(), &credentials, &staging)?;
                }
                std::fs::remove_dir_all(staging)?;
            }
        }

        let output = config.output_file()?;
        let absolute_output = std::path::absolute(&output)?;
        let file = File::create(&output)?;
        let mut archive =
            tar::Builder::new(GzEncoder::new(BufWriter::new(file), Compression::default()));
        archive.follow_symlinks(false);
        for entry in roots {
            append_entry(&mut archive, src, &entry.path(), &absolute_output, self.0)?;
        }
        let mut writer = archive.into_inner()?.finish()?;
        writer.flush()?;
        config.resolve_result(output)
    }
}

fn append_entry<W: Write>(
    archive: &mut tar::Builder<W>,
    root: &Path,
    path: &Path,
    output: &Path,
    platform: Platform,
) -> Result<(), PackageError> {
    // Also skip a previous archive when the output directory is inside the input.
    if std::path::absolute(path)? == output
        || path
            .canonicalize()
            .ok()
            .zip(output.canonicalize().ok())
            .is_some_and(|(a, b)| a == b)
        || (platform == Platform::Windows && crate::windows::common::is_debug_symbols(path))
    {
        return Ok(());
    }
    let name = path
        .strip_prefix(root)
        .map_err(|e| PackageError::General(e.to_string()))?;
    archive.append_path_with_name(path, name)?;
    if path.symlink_metadata()?.is_dir() {
        let mut entries = std::fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            append_entry(archive, root, &entry.path(), output, platform)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;

    fn config(root: &Path, platform: Platform) -> PackageConfig {
        PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "demo".into(),
            app_version: "1.0.0+1".into(),
            build_mode: "release".into(),
            platform,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "tar.gz".into(),
            is_installer: false,
            build_output_dir: root.join("bundle"),
            build_output_files: vec![],
            output_dir: root.join("dist"),
            packaging_root: None,
            environment: Default::default(),
        }
    }

    #[test]
    fn archives_contents_for_each_platform_and_overwrites_existing_output() {
        for platform in [
            Platform::Linux,
            Platform::MacOS,
            Platform::Windows,
            Platform::Web,
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let config = config(tmp.path(), platform);
            let src = &config.build_output_dir;
            std::fs::create_dir_all(src.join("Demo.app/Contents/empty")).unwrap();
            std::fs::write(src.join("Demo.app/Contents/demo"), b"bin").unwrap();
            std::fs::write(src.join(".hidden"), b"hidden").unwrap();
            std::fs::write(src.join("Helper.PDB"), b"symbols").unwrap();
            let packager = TarGzPackager(platform);
            for _ in 0..2 {
                let result = packager.package(&config).unwrap();
                let artifact = &result.artifacts[0];
                assert!(
                    artifact
                        .to_string_lossy()
                        .ends_with(&format!("demo-1.0.0+1-{}.tar.gz", platform.as_str()))
                );
                let mut archive = tar::Archive::new(GzDecoder::new(File::open(artifact).unwrap()));
                let names: Vec<_> = archive
                    .entries()
                    .unwrap()
                    .map(|entry| entry.unwrap().path().unwrap().into_owned())
                    .collect();
                assert!(
                    names
                        .iter()
                        .any(|name| name == Path::new("Demo.app/Contents/demo"))
                );
                assert!(
                    names
                        .iter()
                        .any(|name| name == Path::new("Demo.app/Contents/empty"))
                );
                assert_eq!(
                    names.iter().any(|name| name == Path::new(".hidden")),
                    platform != Platform::MacOS
                );
                assert_eq!(
                    names.iter().any(|name| name == Path::new("Helper.PDB")),
                    !matches!(platform, Platform::MacOS | Platform::Windows)
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn preserves_permissions_and_links_and_does_not_archive_itself() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let tmp = tempfile::tempdir().unwrap();
        let mut config = config(tmp.path(), Platform::Linux);
        let src = &config.build_output_dir;
        std::fs::create_dir_all(src).unwrap();
        std::fs::write(src.join("demo"), b"bin").unwrap();
        std::fs::set_permissions(src.join("demo"), std::fs::Permissions::from_mode(0o755)).unwrap();
        symlink("demo", src.join("link")).unwrap();
        symlink("missing", src.join("dangling")).unwrap();
        symlink(".", src.join("cycle")).unwrap();
        config.output_dir = src.join("dist");
        let packager = TarGzPackager(Platform::Linux);
        for _ in 0..2 {
            let result = packager.package(&config).unwrap();
            let mut archive =
                tar::Archive::new(GzDecoder::new(File::open(&result.artifacts[0]).unwrap()));
            let extracted = tempfile::tempdir().unwrap();
            archive.unpack(extracted.path()).unwrap();
            assert_eq!(
                std::fs::read(extracted.path().join("demo")).unwrap(),
                b"bin"
            );
            assert_eq!(
                std::fs::metadata(extracted.path().join("demo"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o755
            );
            for (name, target) in [("link", "demo"), ("dangling", "missing"), ("cycle", ".")] {
                assert_eq!(
                    std::fs::read_link(extracted.path().join(name)).unwrap(),
                    Path::new(target)
                );
            }
            assert!(
                !extracted
                    .path()
                    .join("dist/1.0.0+1/demo-1.0.0+1-linux.tar.gz")
                    .exists()
            );
        }
    }

    #[test]
    fn rejects_missing_input_and_missing_macos_app() {
        let tmp = tempfile::tempdir().unwrap();
        let config = config(tmp.path(), Platform::MacOS);
        assert!(TarGzPackager(Platform::MacOS).package(&config).is_err());
        std::fs::create_dir_all(&config.build_output_dir).unwrap();
        assert!(TarGzPackager(Platform::MacOS).package(&config).is_err());
    }
}
