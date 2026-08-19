use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Locates an executable in the newest installed build-tools revision.
///
/// The SDK keeps one directory per revision, so the highest version wins —
/// picking whichever directory the filesystem happened to list first would make
/// analysis results depend on the machine.
pub fn build_tool(name: &str) -> Option<PathBuf> {
    for revision in build_tools_revisions() {
        let candidate = revision.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn build_tools_revisions() -> Vec<PathBuf> {
    let Some(sdk_root) = sdk_root() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(sdk_root.join("build-tools")) else {
        return Vec::new();
    };

    let mut revisions: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| !name.to_string_lossy().starts_with('.'))
        })
        .collect();
    revisions.sort_by_key(|revision| std::cmp::Reverse(version_key(revision)));
    revisions
}

pub fn sdk_root() -> Option<PathBuf> {
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .into_iter()
        .filter_map(|variable| env::var(variable).ok())
        .find(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Sorts `36.1.0` above `9.0.0`, which a plain string compare would not.
fn version_key(path: &Path) -> Vec<u32> {
    path.file_name()
        .map(|name| {
            name.to_string_lossy()
                .split('.')
                .map(|part| part.parse::<u32>().unwrap_or(0))
                .collect()
        })
        .unwrap_or_default()
}
