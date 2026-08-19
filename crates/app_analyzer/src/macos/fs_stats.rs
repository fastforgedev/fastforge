use serde_json::{Value, json};
use std::fs;
use std::path::Path;

/// One of the largest files inside a directory tree.
pub struct FileEntry {
    pub path: String,
    pub size_bytes: u64,
}

/// Aggregate statistics for a directory tree.
///
/// Symlinks are counted but never followed: macOS bundles are full of them
/// (`Framework.framework/Versions/Current`, the `/Applications` shortcut in a
/// DMG), and following them would double-count content or loop forever.
#[derive(Default)]
pub struct DirStats {
    pub size_bytes: u64,
    pub file_count: u64,
    pub directory_count: u64,
    pub symlink_count: u64,
    pub largest_files: Vec<FileEntry>,
}

pub fn collect(root: &Path, largest_limit: usize) -> DirStats {
    let mut stats = DirStats::default();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                stats.symlink_count += 1;
                continue;
            }
            if file_type.is_dir() {
                stats.directory_count += 1;
                stack.push(entry.path());
                continue;
            }

            let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            stats.file_count += 1;
            stats.size_bytes += size;

            if largest_limit > 0 {
                let path = entry.path();
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                push_largest(&mut stats.largest_files, relative, size, largest_limit);
            }
        }
    }

    stats
}

/// Total size of a file or of a whole directory tree.
pub fn size_of(path: &Path) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => collect(path, 0).size_bytes,
        Ok(meta) if meta.is_file() => meta.len(),
        _ => 0,
    }
}

pub fn largest_files_json(stats: &DirStats) -> Value {
    Value::Array(
        stats
            .largest_files
            .iter()
            .map(|file| json!({ "path": file.path, "sizeBytes": file.size_bytes }))
            .collect(),
    )
}

/// Keeps `entries` sorted by descending size, capped at `limit`.
fn push_largest(entries: &mut Vec<FileEntry>, path: String, size_bytes: u64, limit: usize) {
    if entries.len() == limit
        && let Some(smallest) = entries.last()
        && smallest.size_bytes >= size_bytes
    {
        return;
    }

    let position = entries
        .iter()
        .position(|entry| entry.size_bytes < size_bytes)
        .unwrap_or(entries.len());
    entries.insert(position, FileEntry { path, size_bytes });
    entries.truncate(limit);
}
