use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use studio_core::api::ApiError;
use studio_core::model::{PackagingFile, PackagingFormat};

const ROOT: &str = ".fastforge/packaging/linux";
const MAX_SIZE: usize = 256 * 1024;

fn io_error(error: std::io::Error) -> ApiError {
    ApiError::internal("PACKAGING_IO_FAILED", error.to_string())
}

/// Refuse symlinks, including dangling ones, before reading or creating files.
/// This applies to every ancestor below the already canonical project root.
fn checked_path(project: &Path, relative: &str) -> Result<PathBuf, ApiError> {
    let mut path = project.to_path_buf();
    for part in Path::new(relative).components() {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(ApiError::bad_request(
                    "INVALID_PATH",
                    "Packaging forms cannot follow symbolic links",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(path)
}

fn relative_path(project: &Path, format: PackagingFormat) -> Result<String, ApiError> {
    match format {
        PackagingFormat::Deb => Ok(format!("{ROOT}/deb/control")),
        PackagingFormat::Pacman => {
            // Same precedence as the native Pacman packager.
            for name in ["PKGINFO", ".PKGINFO"] {
                let relative = format!("{ROOT}/pacman/{name}");
                if checked_path(project, &relative)?.is_file() {
                    return Ok(relative);
                }
            }
            Ok(format!("{ROOT}/pacman/PKGINFO"))
        }
        PackagingFormat::Desktop => {
            let relative = format!("{ROOT}/shared");
            let directory = checked_path(project, &relative)?;
            let entries = match std::fs::read_dir(directory) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(format!("{relative}/${{APP_ID}}.desktop"));
                }
                Err(error) => return Err(io_error(error)),
            };
            let mut matches = Vec::new();
            for entry in entries {
                let entry = entry.map_err(io_error)?;
                let name = entry.file_name();
                if name.to_string_lossy().ends_with(".desktop") {
                    let path = format!("{relative}/{}", name.to_string_lossy());
                    if checked_path(project, &path)?.is_file() {
                        matches.push(path);
                    }
                }
            }
            match matches.len() {
                0 => Ok(format!("{relative}/${{APP_ID}}.desktop")),
                1 => Ok(matches.remove(0)),
                _ => Err(ApiError::conflict(
                    "AMBIGUOUS_PACKAGING_FILE",
                    "The shared directory contains more than one desktop file",
                )),
            }
        }
    }
}

pub fn read(project: &Path, format: PackagingFormat) -> Result<PackagingFile, ApiError> {
    let relative = relative_path(project, format)?;
    let path = checked_path(project, &relative)?;
    let exists = match std::fs::metadata(&path) {
        Ok(metadata) => {
            if metadata.len() > MAX_SIZE as u64 {
                return Err(ApiError::bad_request(
                    "PACKAGING_FILE_TOO_LARGE",
                    "The file exceeds 256 KiB",
                ));
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(io_error(error)),
    };
    let content = if exists {
        std::fs::read_to_string(path).map_err(io_error)?
    } else {
        String::new()
    };
    let etag = format!(
        "{:x}",
        Sha256::digest(format!("{relative}\0{exists}\0{content}"))
    );
    Ok(PackagingFile {
        path: relative,
        exists,
        content,
        etag,
    })
}

/// Called under the server's packaging write lock. Writes are atomic so the CLI
/// and GET requests never read a half-written native template.
pub fn write(
    project: &Path,
    format: PackagingFormat,
    content: &str,
    etag: &str,
) -> Result<PackagingFile, ApiError> {
    if content.len() > MAX_SIZE || content.contains('\0') {
        return Err(ApiError::bad_request(
            "INVALID_PACKAGING_CONTENT",
            "Expected text of at most 256 KiB without NUL bytes",
        ));
    }
    let current = read(project, format)?;
    if current.etag != etag {
        return Err(ApiError::conflict(
            "PACKAGING_FILE_STALE",
            "The file changed on disk. Reload before saving.",
        ));
    }
    let path = checked_path(project, &current.path)?;
    let parent = path.parent().expect("packaging files have a parent");
    std::fs::create_dir_all(parent).map_err(io_error)?;
    let temporary = parent.join(format!(".studio-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        std::fs::write(&temporary, content)?;
        if current.exists {
            std::fs::set_permissions(&temporary, std::fs::metadata(&path)?.permissions())?;
        }
        std::fs::rename(&temporary, &path)
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_file(temporary);
        return Err(io_error(error));
    }
    read(project, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_each_format_and_refuses_stale_writes() {
        let project = tempfile::tempdir().unwrap();
        for format in [
            PackagingFormat::Deb,
            PackagingFormat::Pacman,
            PackagingFormat::Desktop,
        ] {
            let missing = read(project.path(), format).unwrap();
            assert!(!missing.exists);
            let saved = write(project.path(), format, "# ${APP_ID}\n", &missing.etag).unwrap();
            assert!(saved.exists);
            assert_eq!(saved.content, "# ${APP_ID}\n");
            assert_eq!(
                write(project.path(), format, "changed", &missing.etag)
                    .unwrap_err()
                    .status,
                409
            );
            std::fs::write(project.path().join(&saved.path), "external edit").unwrap();
            assert_eq!(
                write(project.path(), format, "changed", &saved.etag)
                    .unwrap_err()
                    .status,
                409
            );
        }
    }

    #[test]
    fn uses_existing_native_names_and_refuses_ambiguous_desktop_files() {
        let project = tempfile::tempdir().unwrap();
        let pacman = project.path().join(format!("{ROOT}/pacman"));
        std::fs::create_dir_all(&pacman).unwrap();
        std::fs::write(pacman.join(".PKGINFO"), "pkgname = app\n").unwrap();
        assert!(
            read(project.path(), PackagingFormat::Pacman)
                .unwrap()
                .path
                .ends_with("/.PKGINFO")
        );
        let shared = project.path().join(format!("{ROOT}/shared"));
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(shared.join("custom.desktop"), "[Desktop Entry]\n").unwrap();
        assert!(
            read(project.path(), PackagingFormat::Desktop)
                .unwrap()
                .path
                .ends_with("/custom.desktop")
        );
        std::fs::write(shared.join("second.desktop"), "").unwrap();
        assert_eq!(
            read(project.path(), PackagingFormat::Desktop)
                .unwrap_err()
                .code,
            "AMBIGUOUS_PACKAGING_FILE"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_ancestors_and_dangling_files() {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), project.path().join(".fastforge")).unwrap();
        assert_eq!(
            read(project.path(), PackagingFormat::Deb).unwrap_err().code,
            "INVALID_PATH"
        );
        std::fs::remove_file(project.path().join(".fastforge")).unwrap();
        let path = project.path().join(format!("{ROOT}/deb"));
        std::fs::create_dir_all(&path).unwrap();
        std::os::unix::fs::symlink(outside.path().join("missing"), path.join("control")).unwrap();
        assert_eq!(
            read(project.path(), PackagingFormat::Deb).unwrap_err().code,
            "INVALID_PATH"
        );
    }
}
