//! Locks in a documented current limitation of `fastforge package`: in a
//! Flutter project, the packager is currently resolved unconditionally via
//! `macos_packager(target)` (see `apps/cli/src/cli/commands/package.rs`), so
//! `--platform android`/`--platform ios` complete the build successfully but
//! then fail at the packaging step with a specific error. This must fail
//! loudly and predictably rather than silently changing behavior later
//! without anyone noticing.

mod support;

use std::fs;
use support::{fixture_dir, run_fastforge};

#[test]
fn flutter_app_android_apk_package_is_unsupported() {
    let dir = fixture_dir("flutter_app");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(
        &dir,
        &["package", "--platform", "android", "--target", "apk"],
    );

    assert!(
        !run.success,
        "expected `fastforge package --platform android` to fail in a Flutter \
         project (packager selection is currently hardcoded to macOS formats); \
         stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr.contains("Unsupported package target"),
        "expected the documented 'Unsupported package target' error, got:\n{}",
        run.stderr
    );

    let _ = fs::remove_dir_all(&dist);
}
