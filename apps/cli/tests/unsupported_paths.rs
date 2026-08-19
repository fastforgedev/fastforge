//! Locks in the error behavior of `fastforge package` for genuinely
//! unsupported `(platform, target)` pairs: the packager registry
//! (`resolve_packager` in `apps/cli/src/cli/commands/package.rs`) must reject
//! mismatched combinations loudly and predictably instead of silently
//! producing the wrong artifact.

mod support;

use std::fs;
use support::{fixture_dir, run_fastforge};

#[test]
fn flutter_app_macos_apk_package_is_unsupported() {
    let dir = fixture_dir("flutter_app");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    // `apk` is an Android format; requesting it for macOS must fail at
    // packager resolution with the documented error message.
    let run = run_fastforge(&dir, &["package", "--platform", "macos", "--target", "apk"]);

    assert!(
        !run.success,
        "expected `fastforge package --platform macos --target apk` to fail \
         (apk is not a macOS package format); stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr.contains("Unsupported package target"),
        "expected the documented 'Unsupported package target' error, got:\n{}",
        run.stderr
    );

    let _ = fs::remove_dir_all(&dist);
}

#[test]
fn package_requires_at_least_one_target() {
    let dir = fixture_dir("flutter_app");

    let run = run_fastforge(&dir, &["package", "--platform", "macos"]);

    assert!(
        !run.success,
        "expected `fastforge package` without --targets to fail; stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr.contains("At least one 'target' must be specified"),
        "expected the missing-target error, got:\n{}",
        run.stderr
    );
}
