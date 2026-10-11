use std::path::{Path, PathBuf};

/// Resolves a fixture under the workspace-root `fixtures/` directory shared by
/// the Build-layer integration tests.
pub fn fixture_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name);
    assert!(
        path.is_dir(),
        "fixture '{name}' not found at {}",
        path.display()
    );
    // canonicalize() adds a Windows verbatim prefix (\\?\), which cmd.exe
    // rejects as its working directory when Flutter's batch script starts.
    // CARGO_MANIFEST_DIR is already absolute; retain a normal Win32 path.
    path
}

/// Changes the process's current working directory for the lifetime of the
/// guard, restoring the previous directory on drop.
///
/// Every `AppBuilder` under test shells out and resolves glob patterns
/// relative to the process's current working directory (mirroring how the
/// real `fastforge` CLI runs from a project root), so integration tests must
/// `chdir` into a fixture before calling a builder. Tests that use this guard
/// must be `#[serial]`-tagged: `set_current_dir` is process-wide, and Rust
/// runs `#[test]`s concurrently by default.
pub struct WorkingDir {
    previous: PathBuf,
}

impl WorkingDir {
    pub fn enter(path: &Path) -> Self {
        let previous = std::env::current_dir().expect("read current dir");
        std::env::set_current_dir(path)
            .unwrap_or_else(|e| panic!("chdir into {}: {e}", path.display()));
        Self { previous }
    }
}

impl Drop for WorkingDir {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.previous).ok();
    }
}
