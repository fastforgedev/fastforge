use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Locates an executable in the newest installed build-tools revision.
///
/// The SDK keeps one directory per revision, so the highest version wins —
/// picking whichever directory the filesystem happened to list first would make
/// analysis results depend on the machine.
pub fn build_tool(name: &str) -> Option<PathBuf> {
    let sdk_root = sdk_root()?;
    build_tool_in_sdk(&sdk_root, name)
}

pub(crate) fn build_tool_in_sdk(sdk_root: &Path, name: &str) -> Option<PathBuf> {
    for revision in build_tools_revisions_in(sdk_root) {
        if let Some(tool) = find_tool_in_revision(&revision, name) {
            return Some(tool);
        }
    }
    None
}

#[cfg(windows)]
fn find_tool_in_revision(revision: &Path, name: &str) -> Option<PathBuf> {
    [
        format!("{name}.exe"),
        format!("{name}.bat"),
        format!("{name}.cmd"),
        name.to_string(),
    ]
    .into_iter()
    .map(|candidate| revision.join(candidate))
    .find(|candidate| candidate.is_file())
}

#[cfg(not(windows))]
fn find_tool_in_revision(revision: &Path, name: &str) -> Option<PathBuf> {
    let candidate = revision.join(name);
    candidate.is_file().then_some(candidate)
}

fn build_tools_revisions_in(sdk_root: &Path) -> Vec<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_tool_from_highest_revision() {
        let temp = tempfile::tempdir().unwrap();
        let older = temp.path().join("build-tools").join("9.0.0");
        let newer = temp.path().join("build-tools").join("36.1.0");
        fs::create_dir_all(&older).unwrap();
        fs::create_dir_all(&newer).unwrap();

        let tool_name = if cfg!(windows) { "aapt2.exe" } else { "aapt2" };
        fs::write(older.join(tool_name), "").unwrap();
        let expected = newer.join(tool_name);
        fs::write(&expected, "").unwrap();

        assert_eq!(build_tool_in_sdk(temp.path(), "aapt2"), Some(expected));
    }

    #[cfg(windows)]
    #[test]
    fn resolves_windows_build_tool_extensions() {
        let temp = tempfile::tempdir().unwrap();
        let rev = temp.path().join("build-tools").join("36.0.0");
        fs::create_dir_all(&rev).unwrap();

        // Extensionless file should lose to `.exe` / `.bat` on Windows.
        fs::write(rev.join("aapt2"), "posix").unwrap();
        let aapt2_exe = rev.join("aapt2.exe");
        fs::write(&aapt2_exe, "exe").unwrap();

        fs::write(rev.join("apksigner"), "posix").unwrap();
        let apksigner_bat = rev.join("apksigner.bat");
        fs::write(&apksigner_bat, "@echo off\r\n").unwrap();

        let d8_cmd = rev.join("d8.cmd");
        fs::write(&d8_cmd, "@echo off\r\n").unwrap();

        // A directory with the tool name must not be returned as an executable.
        fs::create_dir_all(rev.join("zipalign")).unwrap();

        assert_eq!(build_tool_in_sdk(temp.path(), "aapt2"), Some(aapt2_exe));
        assert_eq!(
            build_tool_in_sdk(temp.path(), "apksigner"),
            Some(apksigner_bat)
        );
        assert_eq!(build_tool_in_sdk(temp.path(), "d8"), Some(d8_cmd));
        assert_eq!(build_tool_in_sdk(temp.path(), "zipalign"), None);
    }

    #[cfg(windows)]
    #[test]
    fn executes_resolved_bat_build_tool() {
        let temp = tempfile::tempdir().unwrap();
        let rev = temp.path().join("build-tools").join("36.0.0");
        fs::create_dir_all(&rev).unwrap();
        let bat = rev.join("apksigner.bat");
        fs::write(&bat, "@echo off\r\necho Verifies %1 %2\r\n").unwrap();

        let resolved = build_tool_in_sdk(temp.path(), "apksigner").expect("apksigner.bat");
        assert_eq!(resolved, bat);

        let output = crate::command::run(&resolved.to_string_lossy(), &["verify", "-v"])
            .expect("command::run should execute .bat");
        assert!(output.success);
        assert!(
            output.stdout_text().contains("Verifies verify -v"),
            "unexpected stdout: {}",
            output.stdout_text()
        );
    }
}
