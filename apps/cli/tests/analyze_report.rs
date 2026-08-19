//! Locks in how `fastforge analyze` handles more than one artifact: scanning
//! directories for supported packages, and rendering the HTML report.
//!
//! `.app` bundles are used as fixtures because they are plain directories, so
//! the test needs no SDK tooling — which also makes it macOS-only.
#![cfg(target_os = "macos")]

mod support;

use std::fs;
use std::path::Path;
use support::run_fastforge;
use tempfile::TempDir;

#[test]
fn analyze_scans_a_directory_for_packages() {
    let temp = TempDir::new().expect("temp dir");
    write_app_bundle(
        &temp.path().join("Alpha.app"),
        "dev.fastforge.alpha",
        "1.2.3",
    );
    // A package nested in a subdirectory should be found too.
    write_app_bundle(
        &temp.path().join("nested").join("Beta.app"),
        "dev.fastforge.beta",
        "4.5.6",
    );
    fs::write(temp.path().join("notes.txt"), "not a package").expect("write file");

    let run = run_fastforge(temp.path(), &["analyze", "."]);

    assert!(run.success, "analyze failed: {}", run.stderr);
    let payload: serde_json::Value =
        serde_json::from_str(&run.stdout).expect("analyze should print JSON");
    assert_eq!(payload["artifactCount"], 2);
    let identifiers: Vec<&str> = payload["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .map(|artifact| artifact["identifier"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        identifiers,
        vec!["dev.fastforge.alpha", "dev.fastforge.beta"]
    );
}

#[test]
fn analyze_accepts_several_paths() {
    let temp = TempDir::new().expect("temp dir");
    write_app_bundle(&temp.path().join("one").join("Alpha.app"), "dev.a", "1.0.0");
    write_app_bundle(&temp.path().join("two").join("Beta.app"), "dev.b", "2.0.0");

    let run = run_fastforge(temp.path(), &["analyze", "one", "two"]);

    assert!(run.success, "analyze failed: {}", run.stderr);
    let payload: serde_json::Value = serde_json::from_str(&run.stdout).expect("JSON output");
    assert_eq!(payload["artifactCount"], 2);
}

#[test]
fn analyze_of_a_single_file_keeps_the_flat_payload() {
    let temp = TempDir::new().expect("temp dir");
    let bundle = temp.path().join("Alpha.app");
    write_app_bundle(&bundle, "dev.fastforge.alpha", "1.2.3");

    let run = run_fastforge(temp.path(), &["analyze", "Alpha.app"]);

    assert!(run.success, "analyze failed: {}", run.stderr);
    let payload: serde_json::Value = serde_json::from_str(&run.stdout).expect("JSON output");
    // Naming one artifact still reports just that artifact, as it always has.
    assert_eq!(payload["identifier"], "dev.fastforge.alpha");
    assert!(
        payload.get("artifacts").is_none(),
        "a single named artifact should not be wrapped in a report"
    );
}

#[test]
fn analyze_writes_a_self_contained_html_report() {
    let temp = TempDir::new().expect("temp dir");
    write_app_bundle(
        &temp.path().join("Alpha.app"),
        "dev.fastforge.alpha",
        "1.2.3",
    );
    write_app_bundle(&temp.path().join("Beta.app"), "dev.fastforge.beta", "4.5.6");

    // The format follows the output file's extension.
    let run = run_fastforge(temp.path(), &["analyze", ".", "--output", "report.html"]);

    assert!(run.success, "analyze failed: {}", run.stderr);
    let report = fs::read_to_string(temp.path().join("report.html")).expect("report written");
    assert!(report.starts_with("<!doctype html>"));
    // The page renders itself from the analysis embedded in it.
    assert!(report.contains(r#""artifactCount":2"#));
    assert!(report.contains("dev.fastforge.alpha") && report.contains("dev.fastforge.beta"));
    assert!(
        !report.contains("__FASTFORGE_ANALYSIS_DATA__"),
        "the data placeholder should have been consumed"
    );
    assert!(
        !report.contains("src=\"http") && !report.contains("href=\"http"),
        "the report must not depend on anything it does not carry"
    );
}

#[test]
fn analyze_rejects_a_directory_without_packages() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("notes.txt"), "nothing to analyze").expect("write file");

    let run = run_fastforge(temp.path(), &["analyze", "."]);

    assert!(!run.success, "expected an empty directory to fail");
    assert!(
        run.stderr.contains("No supported packages found"),
        "expected the documented error, got:\n{}",
        run.stderr
    );
}

#[test]
fn analyze_rejects_an_unsupported_file() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("notes.txt"), "nothing to analyze").expect("write file");

    let run = run_fastforge(temp.path(), &["analyze", "notes.txt"]);

    assert!(!run.success, "expected an unsupported file to fail");
    assert!(
        run.stderr.contains("Unsupported file extension"),
        "expected the documented error, got:\n{}",
        run.stderr
    );
}

/// Writes the smallest `.app` bundle the analyzer accepts.
fn write_app_bundle(path: &Path, identifier: &str, version: &str) {
    let contents = path.join("Contents");
    fs::create_dir_all(contents.join("MacOS")).expect("create bundle");
    let name = path
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    fs::write(
        contents.join("Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>{identifier}</string>
  <key>CFBundleName</key><string>{name}</string>
  <key>CFBundleShortVersionString</key><string>{version}</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleExecutable</key><string>{name}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
</dict>
</plist>
"#
        ),
    )
    .expect("write Info.plist");
    fs::write(contents.join("MacOS").join(&name), [0u8; 64]).expect("write executable");
}
