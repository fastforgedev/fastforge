//! The file manifest that makes workspace syncs incremental.
//!
//! The client scans the local project, the agent reports what the remote
//! workspace holds, and only entries that differ are archived and sent. The
//! remote manifest is re-validated against the disk (size, mtime, mode, link
//! target), so files changed on the remote host are sent again.

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::time::UNIX_EPOCH;

/// Project-level ignore file, on top of `.gitignore`. Its `!pattern` lines
/// can re-include files `.gitignore` excludes (e.g. `!android/key.properties`).
pub const IGNORE_FILE: &str = ".fastforgeignore";

/// Directory names that are never synced.
const ALWAYS_EXCLUDED_DIRS: &[&str] = &[".git", ".dart_tool", ".gradle"];

/// Directories, relative to the workspace root, that are never synced: the
/// remote agent's own state (manifest, runs and their outputs). The rest of
/// `.fastforge/` (packaging files, `config.yaml`) is part of the project.
const ALWAYS_EXCLUDED_PATHS: &[&str] = &[".fastforge/remote"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub size: u64,
    /// Modification time in whole seconds since the Unix epoch.
    pub mtime: u64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exec: bool,
    /// Target of a symbolic link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl Entry {
    /// Reads the entry for `path` without following symlinks.
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let meta = std::fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(path)?;
            return Ok(Self {
                size: 0,
                mtime: 0,
                exec: false,
                link: Some(target.to_string_lossy().replace('\\', "/")),
            });
        }
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(Self {
            size: meta.len(),
            mtime,
            exec: is_executable(&meta),
            link: None,
        })
    }
}

#[cfg(unix)]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Files of a workspace keyed by their `/`-separated relative path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub entries: BTreeMap<String, Entry>,
}

/// What has to be sent to bring the remote workspace up to date.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncPlan {
    pub changed: Vec<String>,
    pub deleted: Vec<String>,
}

impl SyncPlan {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.deleted.is_empty()
    }
}

impl Manifest {
    /// Scans `root`, honouring `.gitignore` (also outside git repositories),
    /// `.git/info/exclude` and `.fastforgeignore`. Paths in `extra_excludes`
    /// (relative to `root`, e.g. the output directory) are skipped too.
    pub fn scan(root: &Path, extra_excludes: &[String]) -> Result<Self> {
        let excludes: Vec<String> = extra_excludes
            .iter()
            .map(|p| p.trim_matches('/').to_string())
            .filter(|p| !p.is_empty())
            .collect();
        let walk_root = root.to_path_buf();
        let mut walker = WalkBuilder::new(root);
        walker
            .hidden(false)
            .parents(false)
            .git_global(false)
            .require_git(false)
            .add_custom_ignore_filename(IGNORE_FILE)
            .filter_entry(move |entry| {
                if entry.depth() == 0 {
                    return true;
                }
                let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
                if is_dir
                    && ALWAYS_EXCLUDED_DIRS
                        .iter()
                        .any(|name| entry.file_name() == *name)
                {
                    return false;
                }
                match relative_key(&walk_root, entry.path()) {
                    Some(key) => {
                        !excludes.contains(&key)
                            && !(is_dir && ALWAYS_EXCLUDED_PATHS.contains(&key.as_str()))
                    }
                    None => true,
                }
            });

        let mut entries = BTreeMap::new();
        for item in walker.build() {
            let item = item.with_context(|| format!("Failed to scan {}", root.display()))?;
            let Some(file_type) = item.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                continue;
            }
            let Some(key) = relative_key(root, item.path()) else {
                continue;
            };
            let entry = Entry::from_path(item.path())
                .with_context(|| format!("Failed to read {}", item.path().display()))?;
            entries.insert(key, entry);
        }
        Ok(Self { entries })
    }

    /// The entries of `self` that still match the files under `root`.
    pub fn verified(&self, root: &Path) -> Self {
        let entries = self
            .entries
            .iter()
            .filter(|(key, recorded)| {
                Entry::from_path(&root.join(key)).is_ok_and(|mut actual| {
                    // No executable bit to compare on Windows.
                    if cfg!(not(unix)) {
                        actual.exec = recorded.exec;
                    }
                    &actual == *recorded
                })
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self { entries }
    }

    /// Stable fingerprint, used to detect a workspace changed by someone else
    /// between reading its manifest and syncing.
    pub fn digest(&self) -> String {
        let json = serde_json::to_vec(self).expect("manifest serializes");
        format!("{:016x}", fnv1a(&json))
    }

    /// The plan that turns `remote` into `self`.
    pub fn plan_from(&self, remote: &Manifest) -> SyncPlan {
        let changed = self
            .entries
            .iter()
            .filter(|(key, entry)| remote.entries.get(*key) != Some(*entry))
            .map(|(key, _)| key.clone())
            .collect();
        let deleted = remote
            .entries
            .keys()
            .filter(|key| !self.entries.contains_key(*key))
            .cloned()
            .collect();
        SyncPlan { changed, deleted }
    }
}

/// 64-bit FNV-1a; stable across Rust releases unlike `DefaultHasher`.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn relative_key(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Rejects keys that would escape the workspace (`..`, absolute paths).
pub fn validate_key(key: &str) -> Result<()> {
    let path = Path::new(key);
    if key.is_empty() || !path.components().all(|c| matches!(c, Component::Normal(_))) {
        bail!("Refusing unsafe workspace path `{key}`");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn scan_honours_ignore_files_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "pubspec.yaml", "name: x");
        write(root, "lib/main.dart", "void main() {}");
        write(
            root,
            ".gitignore",
            "build/\nandroid/key.properties\nandroid/local.properties\n",
        );
        write(
            root,
            ".fastforgeignore",
            "!android/key.properties\nsecret.txt\n",
        );
        write(root, "build/app.apk", "x");
        write(root, "android/key.properties", "k");
        write(root, "android/local.properties", "sdk.dir=/x");
        write(root, "secret.txt", "s");
        write(root, ".git/HEAD", "ref");
        write(root, ".dart_tool/x", "x");
        write(root, "dist/1.0/app.apk", "x");
        write(root, "android/app/build.gradle", "g");
        // `.fastforge/` is synced, except the remote agent's state.
        write(root, ".fastforge/config.yaml", "project: {}");
        write(root, ".fastforge/packaging/linux/deb/control", "Package: x");
        write(root, ".fastforge/remote/manifest.json", "{}");
        write(root, ".fastforge/remote/runs/1/out/app.deb", "x");

        let manifest = Manifest::scan(root, &["dist/".to_string()]).unwrap();
        let keys: Vec<&str> = manifest.entries.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            vec![
                ".fastforge/config.yaml",
                ".fastforge/packaging/linux/deb/control",
                ".fastforgeignore",
                ".gitignore",
                "android/app/build.gradle",
                "android/key.properties",
                "lib/main.dart",
                "pubspec.yaml",
            ]
        );
    }

    #[test]
    fn plan_and_verify() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "a.txt", "a");
        write(root, "b.txt", "b");
        let local = Manifest::scan(root, &[]).unwrap();

        let empty = Manifest::default();
        let plan = local.plan_from(&empty);
        assert_eq!(plan.changed, vec!["a.txt", "b.txt"]);
        assert!(plan.deleted.is_empty());
        assert!(local.plan_from(&local).is_empty());

        let mut remote = local.clone();
        remote.entries.insert(
            "old.txt".into(),
            Entry {
                size: 1,
                mtime: 1,
                exec: false,
                link: None,
            },
        );
        remote.entries.get_mut("a.txt").unwrap().size = 99;
        let plan = local.plan_from(&remote);
        assert_eq!(plan.changed, vec!["a.txt"]);
        assert_eq!(plan.deleted, vec!["old.txt"]);

        // A file modified behind the manifest's back drops out of it.
        write(root, "b.txt", "changed!");
        let verified = local.verified(root);
        assert!(verified.entries.contains_key("a.txt"));
        assert!(!verified.entries.contains_key("b.txt"));
        assert_ne!(verified.digest(), local.digest());
    }

    #[test]
    fn rejects_unsafe_keys() {
        assert!(validate_key("lib/main.dart").is_ok());
        assert!(validate_key("../etc/passwd").is_err());
        assert!(validate_key("/etc/passwd").is_err());
        assert!(validate_key("").is_err());
    }
}
