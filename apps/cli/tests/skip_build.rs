//! `fastforge package --skip-build` packages an existing Flutter build output
//! without running `flutter clean` or `flutter build`, so these tests need no
//! Flutter SDK: they lay out the output by hand.

mod support;

use std::fs;
use support::run_fastforge;

fn flutter_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("pubspec.yaml"),
        "name: prebuilt_app\nversion: 1.2.3+4\n",
    )
    .unwrap();
    dir
}

#[test]
fn skip_build_packages_the_existing_output_under_the_given_version() {
    let dir = flutter_project();
    let web = dir.path().join("build/web");
    fs::create_dir_all(&web).unwrap();
    fs::write(web.join("index.html"), "<html></html>").unwrap();

    let run = run_fastforge(
        dir.path(),
        &[
            "package",
            "--platform",
            "web",
            "--targets",
            "zip",
            "--skip-build",
            "--flutter-build-args",
            "build-number=9",
        ],
    );
    assert!(
        run.success,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert!(!run.stderr.contains("flutter clean"), "{}", run.stderr);
    assert!(
        dir.path()
            .join("dist/1.2.3+9/prebuilt_app-1.2.3+9-web.zip")
            .is_file(),
        "stdout: {}",
        run.stdout
    );
}

#[test]
fn skip_build_fails_without_an_earlier_build() {
    let dir = flutter_project();
    let run = run_fastforge(
        dir.path(),
        &[
            "package",
            "--platform",
            "web",
            "--targets",
            "zip",
            "--skip-build",
        ],
    );
    assert!(!run.success);
    assert!(
        run.stderr.contains("build/web") && run.stderr.contains("--skip-build"),
        "{}",
        run.stderr
    );
}
