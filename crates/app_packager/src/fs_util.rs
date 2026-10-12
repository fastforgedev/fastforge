//! Filesystem helpers shared by the packagers: recursive directory copies
//! (Dart's `copyPathSync`) and in-process zipping (Dart's
//! `ZipFileEncoder.zipDirectory`).

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use fastforge_core::PackageError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Recursively copies the *contents* of `src` into `dst`, creating `dst` when
/// needed and overwriting files that already exist there — the behavior of
/// Dart's `copyPathSync(from, to)` (package:io). Unlike `cp -r src dst`, a
/// re-run never nests `src` inside an existing `dst`.
///
/// Hidden and system files are copied too (unlike `xcopy /E`). File
/// permissions are preserved by [`std::fs::copy`]. On Unix, symlinks are
/// recreated as symlinks (like `cp -R`); on other platforms their targets
/// are copied.
pub(crate) fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), PackageError> {
    copy_dir_contents_excluding(src, dst, &|_| false)
}

/// [`copy_dir_contents`] without the files `exclude` matches.
pub(crate) fn copy_dir_contents_excluding(
    src: &Path,
    dst: &Path,
    exclude: &dyn Fn(&Path) -> bool,
) -> Result<(), PackageError> {
    if !src.is_dir() {
        return Err(PackageError::NotFound(format!(
            "Directory not found: {}",
            src.display()
        )));
    }
    std::fs::create_dir_all(dst)?;
    // Nothing to do when copying a directory onto itself (Dart's
    // `_doNothing`).
    if let (Ok(a), Ok(b)) = (src.canonicalize(), dst.canonicalize())
        && a == b
    {
        return Ok(());
    }
    copy_dir_recursive(src, dst, exclude)
}

fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
    exclude: &dyn Fn(&Path) -> bool,
) -> Result<(), PackageError> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let file_type = entry.file_type()?;

        #[cfg(unix)]
        if file_type.is_symlink() {
            let target = std::fs::read_link(&from)?;
            remove_existing(&to)?;
            std::os::unix::fs::symlink(target, &to)?;
            continue;
        }

        // `metadata` follows symlinks (non-Unix platforms copy targets).
        let metadata = std::fs::metadata(&from)?;
        if metadata.is_dir() {
            if to.symlink_metadata().is_ok_and(|m| !m.is_dir()) {
                remove_existing(&to)?;
            }
            std::fs::create_dir_all(&to)?;
            copy_dir_recursive(&from, &to, exclude)?;
        } else {
            let _ = file_type;
            if exclude(&from) {
                continue;
            }
            remove_existing(&to)?;
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Removes a file or symlink at `path` (so read-only files and links are
/// replaced rather than written through). Directories are left alone.
fn remove_existing(path: &Path) -> Result<(), PackageError> {
    match path.symlink_metadata() {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => {
            std::fs::remove_file(path)?;
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

/// Zips the *contents* of `src` (no top-level folder) into `output`,
/// mirroring Dart's `ZipFileEncoder().zipDirectory(dir, filename: ...,
/// followLinks: true)`: symlinks are followed, directory entries are kept,
/// files are deflated and carry their Unix permissions and modification
/// time, and an existing `output` is overwritten.
pub(crate) fn zip_dir_contents(src: &Path, output: &Path) -> Result<(), PackageError> {
    zip_dir_contents_excluding(src, output, &|_| false)
}

/// [`zip_dir_contents`] without the files `exclude` matches.
pub(crate) fn zip_dir_contents_excluding(
    src: &Path,
    output: &Path,
    exclude: &dyn Fn(&Path) -> bool,
) -> Result<(), PackageError> {
    if !src.is_dir() {
        return Err(PackageError::NotFound(format!(
            "Directory not found: {}",
            src.display()
        )));
    }
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    // Build the entry list before creating the archive so an output file
    // located inside `src` is never added to itself.
    let mut entries = Vec::new();
    let mut visited = HashSet::new();
    collect_entries(src, Path::new(""), &mut entries, &mut visited)?;
    entries.retain(|entry| entry.is_dir || !exclude(&entry.path));

    let file = File::create(output)?;
    let output_canonical = output.canonicalize().ok();
    let mut zip = ZipWriter::new(BufWriter::new(file));
    for entry in entries {
        let metadata = std::fs::metadata(&entry.path)?;
        let mut options =
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        if let Some(mode) = unix_mode(&metadata) {
            options = options.unix_permissions(mode);
        }
        if let Some(time) = metadata.modified().ok().and_then(zip_date_time) {
            options = options.last_modified_time(time);
        }
        if entry.is_dir {
            zip.add_directory(format!("{}/", entry.name), options)
                .map_err(zip_error)?;
            continue;
        }
        if output_canonical.is_some() && entry.path.canonicalize().ok() == output_canonical {
            continue;
        }
        if metadata.len() >= u32::MAX as u64 {
            options = options.large_file(true);
        }
        zip.start_file(entry.name.as_str(), options)
            .map_err(zip_error)?;
        let mut reader = File::open(&entry.path)?;
        std::io::copy(&mut reader, &mut zip)?;
    }
    let mut writer = zip.finish().map_err(zip_error)?;
    writer.flush()?;
    Ok(())
}

struct ZipEntry {
    path: PathBuf,
    /// Archive path using `/` separators.
    name: String,
    is_dir: bool,
}

fn collect_entries(
    dir: &Path,
    prefix: &Path,
    entries: &mut Vec<ZipEntry>,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), PackageError> {
    // Guard against symlink cycles when following links.
    if let Ok(canonical) = dir.canonicalize()
        && !visited.insert(canonical)
    {
        return Ok(());
    }
    let mut children: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    children.sort_by_key(|e| e.file_name());
    for child in children {
        let path = child.path();
        let relative = prefix.join(child.file_name());
        let name = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        // Follow symlinks; skip dangling ones.
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            entries.push(ZipEntry {
                path: path.clone(),
                name,
                is_dir: true,
            });
            collect_entries(&path, &relative, entries, visited)?;
        } else {
            entries.push(ZipEntry {
                path,
                name,
                is_dir: false,
            });
        }
    }
    Ok(())
}

#[cfg(unix)]
fn unix_mode(metadata: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(metadata.permissions().mode() & 0o7777)
}

#[cfg(not(unix))]
fn unix_mode(_metadata: &std::fs::Metadata) -> Option<u32> {
    None
}

/// Converts a `SystemTime` to a zip (MS-DOS) timestamp in UTC.
fn zip_date_time(time: std::time::SystemTime) -> Option<zip::DateTime> {
    let secs = time.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as u16;
    zip::DateTime::from_date_and_time(
        year,
        month,
        day,
        (rem / 3600) as u8,
        ((rem % 3600) / 60) as u8,
        (rem % 60) as u8,
    )
    .ok()
}

fn zip_error(err: zip::result::ZipError) -> PackageError {
    PackageError::General(format!("Failed to write zip archive: {}", err))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_zip_names(path: &Path) -> Vec<String> {
        let file = File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

    #[test]
    fn copy_merges_contents_without_nesting() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("bundle");
        std::fs::create_dir_all(src.join("lib")).unwrap();
        std::fs::write(src.join("app"), b"bin").unwrap();
        std::fs::write(src.join(".hidden"), b"h").unwrap();
        std::fs::write(src.join("lib/libapp.so"), b"so").unwrap();

        let dst = tmp.path().join("out");
        copy_dir_contents(&src, &dst).unwrap();
        // Second run must not nest `bundle` inside `out`.
        std::fs::write(src.join("app"), b"bin2").unwrap();
        copy_dir_contents(&src, &dst).unwrap();

        assert_eq!(std::fs::read(dst.join("app")).unwrap(), b"bin2");
        assert!(dst.join(".hidden").exists());
        assert!(dst.join("lib/libapp.so").exists());
        assert!(!dst.join("bundle").exists());
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_permissions_and_symlinks() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let bin = src.join("run.sh");
        std::fs::write(&bin, b"#!/bin/sh").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::os::unix::fs::symlink("run.sh", src.join("link")).unwrap();

        let dst = tmp.path().join("dst");
        copy_dir_contents(&src, &dst).unwrap();
        copy_dir_contents(&src, &dst).unwrap();
        let mode = std::fs::metadata(dst.join("run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        assert_eq!(
            std::fs::read_link(dst.join("link")).unwrap(),
            PathBuf::from("run.sh")
        );
    }

    #[test]
    fn zip_contains_contents_without_top_level_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("bundle");
        std::fs::create_dir_all(src.join("data/empty")).unwrap();
        std::fs::write(src.join("app"), b"bin").unwrap();
        std::fs::write(src.join("data/icudtl.dat"), b"dat").unwrap();

        // Relative-looking nested output path (outside src).
        let output = tmp.path().join("dist/1.0.0/app.zip");
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(&output, b"stale").unwrap();
        zip_dir_contents(&src, &output).unwrap();

        let names = read_zip_names(&output);
        assert!(names.contains(&"app".to_string()));
        assert!(names.contains(&"data/".to_string()));
        assert!(names.contains(&"data/empty/".to_string()));
        assert!(names.contains(&"data/icudtl.dat".to_string()));
        assert!(!names.iter().any(|n| n.starts_with("bundle")));
    }

    #[cfg(unix)]
    #[test]
    fn zip_preserves_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("bundle");
        std::fs::create_dir_all(&src).unwrap();
        let bin = src.join("app");
        std::fs::write(&bin, b"bin").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let output = tmp.path().join("app.zip");
        zip_dir_contents(&src, &output).unwrap();

        let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
        let entry = archive.by_name("app").unwrap();
        assert_eq!(entry.unix_mode().unwrap() & 0o777, 0o755);
    }

    #[test]
    fn zip_date_time_conversion() {
        let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        let dt = zip_date_time(t).unwrap();
        // 2023-11-14 22:13:20 UTC
        assert_eq!(
            (
                dt.year(),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second()
            ),
            (2023, 11, 14, 22, 13, 20)
        );
    }
}
