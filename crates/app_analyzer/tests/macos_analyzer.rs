//! Covers the detail the macOS analyzers extract from a `.app` bundle and from
//! the DMG that ships it. The fixtures are synthesized rather than checked in:
//! a bundle is just a directory layout, and building it in the test keeps the
//! expectations and the input side by side.
#![cfg(target_os = "macos")]

use fastforge_app_analyzer::{AnalyzeConfig, AppAnalyzer, MacOSAppAnalyzer, MacOSDmgAnalyzer};
use serde_json::Value;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn app_analysis_reports_bundle_identity_and_layout() {
    let temp = TempDir::new().expect("temp dir");
    let app_path = build_app_bundle(temp.path());

    let data = analyze_app(&app_path);

    assert_eq!(data["platform"], "macos");
    assert_eq!(data["format"], "app");
    assert_eq!(data["identifier"], "dev.fastforge.fixture");
    assert_eq!(data["name"], "Fixture App");
    assert_eq!(data["version"], "1.2.3");
    assert_eq!(data["buildNumber"], "42");
    assert_eq!(data["executable"], "Fixture");
    assert_eq!(data["bundleType"], "APPL");
    assert_eq!(data["minOSVersion"], "12.0");
    assert_eq!(data["category"], "public.app-category.developer-tools");
    assert_eq!(data["fileName"], "Fixture.app");
    assert_eq!(
        data["path"],
        Value::String(
            fs::canonicalize(&app_path)
                .expect("canonical path")
                .to_string_lossy()
                .into_owned()
        )
    );

    // Toolchain keys Xcode stamps into every Info.plist it writes.
    assert_eq!(data["buildInfo"]["sdk"], "macosx14.2");
    assert_eq!(data["buildInfo"]["xcodeBuild"], "15C500b");

    assert_eq!(
        data["localizations"],
        Value::Array(vec!["en".into(), "zh-Hans".into()])
    );
    assert!(data["sizeBytes"].as_u64().unwrap() > 0);
    assert!(data["fileCount"].as_u64().unwrap() >= 4);
    assert!(
        data["sizeBreakdown"]["Frameworks"].as_u64().unwrap() > 0,
        "expected the frameworks directory in the size breakdown: {}",
        data["sizeBreakdown"]
    );
    assert!(
        !data["largestFiles"].as_array().unwrap().is_empty(),
        "expected the biggest files to be listed"
    );
}

#[test]
fn app_analysis_reads_architectures_from_the_mach_o_header() {
    let temp = TempDir::new().expect("temp dir");
    let app_path = build_app_bundle(temp.path());

    let data = analyze_app(&app_path);

    assert_eq!(
        data["architectures"],
        Value::Array(vec!["x86_64".into(), "arm64".into()]),
        "universal binary slices should be read straight from the fat header"
    );
    assert_eq!(data["universal"], Value::Bool(true));
    assert!(data["executableSizeBytes"].as_u64().unwrap() > 0);
}

#[test]
fn app_analysis_detects_the_flutter_runtime_and_embedded_components() {
    let temp = TempDir::new().expect("temp dir");
    let app_path = build_app_bundle(temp.path());

    let data = analyze_app(&app_path);

    assert_eq!(data["techStack"]["runtime"], "flutter");
    let flutter = &data["techStack"]["flutter"];
    assert_eq!(flutter["engineRevision"], "abc123def456");
    assert_eq!(flutter["buildMode"], "release");
    assert_eq!(
        flutter["aot"],
        Value::Bool(true),
        "no kernel blob means an AOT (release) build"
    );
    assert!(flutter["assets"]["fileCount"].as_u64().unwrap() >= 1);

    let frameworks = data["components"]["frameworks"].as_array().unwrap();
    let names: Vec<&str> = frameworks
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["App.framework", "FlutterMacOS.framework"]);
    assert_eq!(
        frameworks[1]["identifier"], "io.flutter.flutter-macos",
        "framework metadata should come from its own Info.plist"
    );
}

#[test]
fn app_analysis_reports_declared_capabilities() {
    let temp = TempDir::new().expect("temp dir");
    let app_path = build_app_bundle(temp.path());

    let data = analyze_app(&app_path);

    let url_types = data["urlSchemes"].as_array().unwrap();
    assert_eq!(url_types.len(), 1);
    assert_eq!(url_types[0]["name"], "dev.fastforge.fixture.url");
    assert_eq!(
        url_types[0]["schemes"],
        Value::Array(vec!["fastforge".into()])
    );

    let document_types = data["documentTypes"].as_array().unwrap();
    assert_eq!(document_types[0]["name"], "Fixture Document");
    assert_eq!(
        document_types[0]["extensions"],
        Value::Array(vec!["fixture".into()])
    );

    assert_eq!(
        data["privacyUsageDescriptions"]["NSCameraUsageDescription"],
        "Scan setup codes."
    );
}

#[test]
fn app_analysis_reports_an_unsigned_bundle_as_unsigned() {
    let temp = TempDir::new().expect("temp dir");
    let app_path = build_app_bundle(temp.path());

    let data = analyze_app(&app_path);

    assert_eq!(
        data["codeSignature"]["signed"],
        Value::Bool(false),
        "a synthesized bundle carries no signature: {}",
        data["codeSignature"]
    );
}

#[test]
fn app_analysis_rejects_paths_that_are_not_bundles() {
    let temp = TempDir::new().expect("temp dir");
    let not_a_bundle = temp.path().join("Fixture.zip");
    fs::create_dir_all(&not_a_bundle).expect("create directory");

    let error = MacOSAppAnalyzer::new()
        .analyze(AnalyzeConfig::new(
            not_a_bundle.to_string_lossy().into_owned(),
        ))
        .expect_err("expected a non-.app directory to be rejected");

    assert!(
        error.to_string().contains(".app"),
        "error should name the expected bundle format, got: {error}"
    );
}

#[test]
fn dmg_analysis_reports_image_volume_and_bundle() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("volume");
    fs::create_dir_all(&source).expect("create volume source");
    build_app_bundle(&source);
    unix_fs::symlink("/Applications", source.join("Applications")).expect("applications symlink");

    let dmg_path = temp.path().join("Fixture.dmg");
    create_dmg(&source, "Fixture Installer", &dmg_path);

    let result = MacOSDmgAnalyzer::new()
        .analyze(AnalyzeConfig::new(dmg_path.to_string_lossy().into_owned()))
        .expect("dmg analysis should succeed");
    let data = result.data;

    assert_eq!(data["platform"], "macos");
    assert_eq!(data["format"], "dmg");
    // Identity is mirrored from the bundle inside for backwards compatibility.
    assert_eq!(data["identifier"], "dev.fastforge.fixture");
    assert_eq!(data["name"], "Fixture App");
    assert_eq!(data["version"], "1.2.3");
    assert_eq!(data["buildNumber"], "42");
    assert_eq!(data["fileName"], "Fixture.dmg");
    assert_eq!(
        data["sizeBytes"].as_u64().unwrap(),
        fs::metadata(&dmg_path).unwrap().len()
    );
    assert_eq!(
        data["sha256"].as_str().unwrap().len(),
        64,
        "sha256 should be reported as 64 hex characters"
    );

    assert_eq!(data["diskImage"]["format"], "UDZO");
    assert_eq!(
        data["diskImage"]["formatDescription"],
        "UDIF read-only, zlib-compressed"
    );
    assert_eq!(data["diskImage"]["compressed"], Value::Bool(true));
    assert_eq!(data["diskImage"]["encrypted"], Value::Bool(false));
    assert!(data["diskImage"]["totalBytes"].as_u64().unwrap() > 0);

    assert_eq!(data["volume"]["name"], "Fixture Installer");
    assert_eq!(
        data["volume"]["hasApplicationsSymlink"],
        Value::Bool(true),
        "the drag-to-install shortcut should be detected"
    );
    let items = data["volume"]["items"].as_array().unwrap();
    let app_item = items
        .iter()
        .find(|item| item["kind"] == "app")
        .expect("the bundle should be listed in the volume contents");
    assert_eq!(app_item["name"], "Fixture.app");

    // The bundle is reported relative to the volume, not to the temporary
    // mount point it was read through.
    assert_eq!(data["app"]["path"], "Fixture.app");
    assert_eq!(data["app"]["techStack"]["runtime"], "flutter");
    assert_eq!(
        data["app"]["architectures"],
        Value::Array(vec!["x86_64".into(), "arm64".into()])
    );
}

#[test]
fn dmg_analysis_rejects_a_missing_image() {
    let temp = TempDir::new().expect("temp dir");
    let missing = temp.path().join("nope.dmg");

    let error = MacOSDmgAnalyzer::new()
        .analyze(AnalyzeConfig::new(missing.to_string_lossy().into_owned()))
        .expect_err("expected a missing disk image to be rejected");

    assert!(
        error.to_string().contains("not found"),
        "error should say the image is missing, got: {error}"
    );
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn analyze_app(app_path: &Path) -> Value {
    MacOSAppAnalyzer::new()
        .analyze(AnalyzeConfig::new(app_path.to_string_lossy().into_owned()))
        .expect("app analysis should succeed")
        .data
}

/// Writes a minimal but realistic Flutter-style `.app` bundle into `parent`.
fn build_app_bundle(parent: &Path) -> PathBuf {
    let app_path = parent.join("Fixture.app");
    let contents = app_path.join("Contents");

    write_file(&contents.join("Info.plist"), INFO_PLIST.as_bytes());
    write_file(&contents.join("MacOS").join("Fixture"), &universal_mach_o());
    write_file(
        &contents
            .join("Resources")
            .join("en.lproj")
            .join("Main.strings"),
        b"",
    );
    write_file(
        &contents
            .join("Resources")
            .join("zh-Hans.lproj")
            .join("Main.strings"),
        b"",
    );

    let frameworks = contents.join("Frameworks");
    write_file(
        &frameworks
            .join("FlutterMacOS.framework")
            .join("Resources")
            .join("Info.plist"),
        FLUTTER_FRAMEWORK_PLIST.as_bytes(),
    );
    write_file(
        &frameworks
            .join("App.framework")
            .join("Resources")
            .join("Info.plist"),
        APP_FRAMEWORK_PLIST.as_bytes(),
    );
    write_file(
        &frameworks
            .join("App.framework")
            .join("Resources")
            .join("flutter_assets")
            .join("AssetManifest.bin"),
        &[0u8; 64],
    );

    app_path
}

fn create_dmg(source: &Path, volume_name: &str, output: &Path) {
    let status = Command::new("hdiutil")
        .args([
            "create",
            "-volname",
            volume_name,
            "-srcfolder",
            &source.to_string_lossy(),
            "-ov",
            "-format",
            "UDZO",
            &output.to_string_lossy(),
        ])
        .output()
        .expect("hdiutil create should run");
    assert!(
        status.status.success(),
        "hdiutil create failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn write_file(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create directories");
    fs::write(path, contents).expect("write file");
}

/// A fat Mach-O header advertising an `x86_64` and an `arm64` slice. Only the
/// header is read during analysis, so no real code is needed.
fn universal_mach_o() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xcafe_babeu32.to_be_bytes()); // FAT_MAGIC
    bytes.extend_from_slice(&2u32.to_be_bytes()); // slice count
    for (cpu_type, cpu_subtype) in [(0x0100_0007u32, 3u32), (0x0100_000cu32, 0u32)] {
        bytes.extend_from_slice(&cpu_type.to_be_bytes());
        bytes.extend_from_slice(&cpu_subtype.to_be_bytes());
        bytes.extend_from_slice(&4096u32.to_be_bytes()); // offset
        bytes.extend_from_slice(&0u32.to_be_bytes()); // size
        bytes.extend_from_slice(&14u32.to_be_bytes()); // align
    }
    bytes.resize(4096, 0);
    bytes
}

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>dev.fastforge.fixture</string>
  <key>CFBundleName</key><string>Fixture</string>
  <key>CFBundleDisplayName</key><string>Fixture App</string>
  <key>CFBundleShortVersionString</key><string>1.2.3</string>
  <key>CFBundleVersion</key><string>42</string>
  <key>CFBundleExecutable</key><string>Fixture</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleSupportedPlatforms</key><array><string>MacOSX</string></array>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHumanReadableCopyright</key><string>Copyright fastforge</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSCameraUsageDescription</key><string>Scan setup codes.</string>
  <key>DTSDKName</key><string>macosx14.2</string>
  <key>DTXcode</key><string>1520</string>
  <key>DTXcodeBuild</key><string>15C500b</string>
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleURLName</key><string>dev.fastforge.fixture.url</string>
      <key>CFBundleURLSchemes</key><array><string>fastforge</string></array>
    </dict>
  </array>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Fixture Document</string>
      <key>CFBundleTypeExtensions</key><array><string>fixture</string></array>
      <key>CFBundleTypeRole</key><string>Editor</string>
    </dict>
  </array>
</dict>
</plist>
"#;

const FLUTTER_FRAMEWORK_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>io.flutter.flutter-macos</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1.0</string>
  <key>FlutterEngine</key><string>abc123def456</string>
  <key>BuildMode</key><string>release</string>
</dict>
</plist>
"#;

const APP_FRAMEWORK_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>io.flutter.flutter.app</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1.0</string>
</dict>
</plist>
"#;
