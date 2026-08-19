//! Real end-to-end tests for `fastforge package` / `fastforge workflow run`
//! against the fixtures under `fixtures/`. Each test spawns the real compiled
//! `fastforge` binary and asserts on real produced artifacts, exercising the
//! same routing functions (`package_native_*` / `package_flutter_artifact`)
//! that back both `fastforge package` and the `fastforge/package` workflow
//! action (see `apps/cli/src/cli/commands/{package,workflow}.rs`).

mod support;

use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use support::{fixture_dir, run_fastforge};

fn find_files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(find_files_with_ext(&path, ext));
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(path);
        }
    }
    out
}

#[test]
#[serial]
fn native_android_apk_via_plain_package_command() {
    let dir = fixture_dir("native_android");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(
        &dir,
        &["package", "--platform", "android", "--target", "apk"],
    );
    assert!(
        run.success,
        "fastforge package failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let apks = find_files_with_ext(&dist, "apk");
    assert!(
        !apks.is_empty(),
        "expected an .apk under {}",
        dist.display()
    );
    let name = apks[0].file_name().unwrap().to_string_lossy().to_string();
    assert!(
        name.contains("dev.fastforge.native_android"),
        "artifact name should be derived from the real applicationId: {name}"
    );
    assert!(
        !name.contains("ownCal"),
        "app name must not be hardcoded to a placeholder: {name}"
    );

    let _ = fs::remove_dir_all(&dist);
}

#[test]
#[serial]
fn native_android_aab_via_plain_package_command() {
    let dir = fixture_dir("native_android");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(
        &dir,
        &["package", "--platform", "android", "--target", "aab"],
    );
    assert!(
        run.success,
        "fastforge package failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let aabs = find_files_with_ext(&dist, "aab");
    assert!(
        !aabs.is_empty(),
        "expected an .aab under {}",
        dist.display()
    );

    let _ = fs::remove_dir_all(&dist);
}

#[test]
#[serial]
fn native_android_flavors_via_workflow() {
    let dir = fixture_dir("native_android_flavors");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(
        &dir,
        &[
            "workflow",
            "run",
            "--file",
            ".fastforge/workflows/package.yml",
            "--input",
            "flavor=dev",
        ],
    );
    assert!(
        run.success,
        "fastforge workflow run failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let apks = find_files_with_ext(&dist, "apk");
    assert!(
        apks.iter()
            .any(|p| p.file_name().unwrap().to_string_lossy().contains("dev")),
        "expected a dev-flavor .apk: {:?}",
        apks
    );
    let aabs = find_files_with_ext(&dist, "aab");
    assert!(
        aabs.iter()
            .any(|p| p.file_name().unwrap().to_string_lossy().contains("dev")),
        "expected a dev-flavor .aab: {:?}",
        aabs
    );

    let _ = fs::remove_dir_all(&dist);
}

#[test]
#[serial]
fn native_ios_ipa_via_workflow() {
    let dir = fixture_dir("native_ios");
    let dist = dir.join("dist");
    let build = dir.join("build");
    let _ = fs::remove_dir_all(&dist);
    let _ = fs::remove_dir_all(&build);

    let run = run_fastforge(
        &dir,
        &[
            "workflow",
            "run",
            "--file",
            ".fastforge/workflows/package.yml",
        ],
    );
    assert!(
        run.success,
        "fastforge workflow run failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let ipas = find_files_with_ext(&dist, "ipa");
    assert!(!ipas.is_empty(), "expected a .ipa under {}", dist.display());

    let _ = fs::remove_dir_all(&dist);
    let _ = fs::remove_dir_all(&build);
}

#[test]
#[serial]
fn native_macos_zip_via_workflow() {
    let dir = fixture_dir("native_macos");
    let dist = dir.join("dist");
    let build = dir.join("build");
    let _ = fs::remove_dir_all(&dist);
    let _ = fs::remove_dir_all(&build);

    let run = run_fastforge(
        &dir,
        &[
            "workflow",
            "run",
            "--file",
            ".fastforge/workflows/package.yml",
        ],
    );
    assert!(
        run.success,
        "fastforge workflow run failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let zips = find_files_with_ext(&dist, "zip");
    assert!(!zips.is_empty(), "expected a .zip under {}", dist.display());

    let _ = fs::remove_dir_all(&dist);
    let _ = fs::remove_dir_all(&build);
}

#[test]
#[serial]
fn flutter_app_macos_dmg() {
    let dir = fixture_dir("flutter_app");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(&dir, &["package", "--platform", "macos", "--target", "dmg"]);
    assert!(
        run.success,
        "fastforge package failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let dmgs = find_files_with_ext(&dist, "dmg");
    assert!(!dmgs.is_empty(), "expected a .dmg under {}", dist.display());

    let _ = fs::remove_dir_all(&dist);
}

#[test]
#[serial]
fn flutter_app_macos_pkg() {
    let dir = fixture_dir("flutter_app");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(&dir, &["package", "--platform", "macos", "--target", "pkg"]);
    assert!(
        run.success,
        "fastforge package failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let pkgs = find_files_with_ext(&dist, "pkg");
    assert!(!pkgs.is_empty(), "expected a .pkg under {}", dist.display());

    let _ = fs::remove_dir_all(&dist);
}

#[test]
#[serial]
fn flutter_app_android_apk() {
    let dir = fixture_dir("flutter_app");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(
        &dir,
        &["package", "--platform", "android", "--target", "apk"],
    );
    assert!(
        run.success,
        "fastforge package failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let apks = find_files_with_ext(&dist, "apk");
    assert!(!apks.is_empty(), "expected an .apk under {}", dist.display());

    let _ = fs::remove_dir_all(&dist);
}

#[test]
#[serial]
fn flutter_app_macos_multiple_targets() {
    let dir = fixture_dir("flutter_app");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(
        &dir,
        &["package", "--platform", "macos", "--targets", "dmg,zip"],
    );
    assert!(
        run.success,
        "fastforge package failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let dmgs = find_files_with_ext(&dist, "dmg");
    let zips = find_files_with_ext(&dist, "zip");
    assert!(!dmgs.is_empty(), "expected a .dmg under {}", dist.display());
    assert!(!zips.is_empty(), "expected a .zip under {}", dist.display());

    let _ = fs::remove_dir_all(&dist);
}

#[test]
#[serial]
fn flutter_app_macos_zip() {
    let dir = fixture_dir("flutter_app");
    let dist = dir.join("dist");
    let _ = fs::remove_dir_all(&dist);

    let run = run_fastforge(&dir, &["package", "--platform", "macos", "--target", "zip"]);
    assert!(
        run.success,
        "fastforge package failed:\nstdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );

    let zips = find_files_with_ext(&dist, "zip");
    assert!(!zips.is_empty(), "expected a .zip under {}", dist.display());

    let _ = fs::remove_dir_all(&dist);
}
