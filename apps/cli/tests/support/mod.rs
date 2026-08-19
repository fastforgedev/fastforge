//! Each test binary compiles this module separately and uses the part it needs,
//! so some helpers are unused in any given one.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Resolves a fixture under the workspace-root `fixtures/` directory shared by
/// the CLI integration tests.
pub fn fixture_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name);
    path.canonicalize()
        .unwrap_or_else(|e| panic!("fixture '{name}' not found at {}: {e}", path.display()))
}

/// Captured result of running the real `fastforge` binary as a subprocess.
pub struct CliRun {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl CliRun {
    fn from_output(output: Output) -> Self {
        Self {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

/// Runs the compiled `fastforge` binary with the given args, with its working
/// directory set to `cwd`. Each invocation spawns an independent subprocess,
/// so unlike the crate-level Build-layer tests it doesn't need a CWD guard.
/// It does *not* by itself make concurrent tests safe, though: any two tests
/// pointed at the same fixture directory (e.g. two `flutter_app` package
/// targets) still race on that directory's build output and on shared
/// Gradle/Xcode system state, and must be `#[serial]`-tagged.
pub fn run_fastforge(cwd: &Path, args: &[&str]) -> CliRun {
    let output = Command::new(env!("CARGO_BIN_EXE_fastforge"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn fastforge: {e}"));
    CliRun::from_output(output)
}
