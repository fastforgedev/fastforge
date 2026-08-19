//! Covers what the IPA analyzer reads out of an iOS app: its identity, the
//! libraries its binary links, the components it ships, and the profile it was
//! signed with. The fixture is synthesized so the expectations sit next to the
//! input that produces them.

mod support;

use fastforge_app_analyzer::{AnalyzeConfig, AppAnalyzer, IOSIpaAnalyzer};
use serde_json::Value;
use std::path::{Path, PathBuf};
use support::{thin_mach_o, write_zip};
use tempfile::TempDir;

const CPU_TYPE_ARM64: u32 = 0x0100_000c;
const PLATFORM_IOS: u32 = 2;

#[test]
fn ipa_analysis_reports_bundle_identity() {
    let temp = TempDir::new().expect("temp dir");
    let data = analyze(&build_ipa(temp.path()));

    assert_eq!(data["platform"], "ios");
    assert_eq!(data["format"], "ipa");
    assert_eq!(data["identifier"], "dev.fastforge.fixture");
    assert_eq!(data["name"], "Fixture App");
    assert_eq!(data["version"], "3.1.0");
    // `CFBundleVersion` is not always numeric, so it is reported verbatim.
    assert_eq!(data["buildNumber"], "77");
    assert_eq!(data["bundlePath"], "Payload/Fixture.app");
    assert_eq!(data["minOSVersion"], "15.0");
    assert_eq!(
        data["deviceFamilies"],
        Value::Array(vec!["iPhone".into(), "iPad".into()]),
        "UIDeviceFamily numbers should be reported as device names"
    );
    assert_eq!(data["architectures"], Value::Array(vec!["arm64".into()]));
    assert_eq!(data["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(data["codeSignature"]["signed"], Value::Bool(true));
}

#[test]
fn ipa_analysis_reads_the_tech_stack_from_the_app_binary() {
    let temp = TempDir::new().expect("temp dir");
    let data = analyze(&build_ipa(temp.path()));
    let stack = &data["techStack"];

    assert_eq!(stack["runtime"], "flutter");
    assert_eq!(stack["flutter"]["aot"], Value::Bool(true));
    assert_eq!(
        stack["flutter"]["plugins"],
        Value::Array(vec!["url_launcher_ios".into()]),
        "plugin frameworks are named after their pub package"
    );
    assert_eq!(
        stack["languages"],
        Value::Array(vec!["Swift".into(), "Objective-C".into(), "C++".into()])
    );
    assert_eq!(
        stack["uiToolkits"],
        Value::Array(vec!["SwiftUI".into(), "UIKit".into()])
    );
    assert_eq!(
        stack["systemFrameworks"],
        Value::Array(vec!["SwiftUI".into(), "UIKit".into()])
    );
    assert_eq!(
        stack["embeddedFrameworks"],
        Value::Array(vec![
            "Flutter".into(),
            "Sentry".into(),
            "url_launcher_ios".into()
        ])
    );

    // Straight out of LC_BUILD_VERSION, so it describes the build rather than
    // what Info.plist claims.
    assert_eq!(stack["toolchain"]["platform"], "iOS");
    assert_eq!(stack["toolchain"]["minOS"], "15.0");
    assert_eq!(stack["toolchain"]["sdk"], "17.2");
    assert_eq!(stack["toolchain"]["clang"], "1500.3.9");
    assert_eq!(stack["toolchain"]["swift"], "5.9");

    assert_eq!(stack["thirdPartySdks"][0]["name"], "Sentry");
    assert_eq!(stack["thirdPartySdks"][0]["category"], "crash-reporting");
}

#[test]
fn ipa_analysis_lists_embedded_components() {
    let temp = TempDir::new().expect("temp dir");
    let data = analyze(&build_ipa(temp.path()));

    let frameworks = data["components"]["frameworks"].as_array().unwrap();
    let names: Vec<&str> = frameworks
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            "Flutter.framework",
            "Sentry.framework",
            "url_launcher_ios.framework"
        ]
    );

    let extension = &data["components"]["appExtensions"][0];
    assert_eq!(extension["name"], "Widget.appex");
    assert_eq!(extension["identifier"], "dev.fastforge.fixture.widget");
    assert_eq!(
        extension["extensionPoint"], "com.apple.widgetkit-extension",
        "the extension point says what kind of extension it is"
    );

    assert!(data["contents"]["entryCount"].as_u64().unwrap() > 0);
    assert!(
        !data["contents"]["largestEntries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn ipa_analysis_reports_declared_capabilities() {
    let temp = TempDir::new().expect("temp dir");
    let data = analyze(&build_ipa(temp.path()));
    let capabilities = &data["capabilities"];

    assert_eq!(capabilities["urlSchemes"][0]["name"], "dev.fastforge.url");
    assert_eq!(
        capabilities["backgroundModes"],
        Value::Array(vec!["remote-notification".into(), "audio".into()])
    );
    assert_eq!(capabilities["allowsArbitraryLoads"], Value::Bool(true));
    assert_eq!(capabilities["encryptionExempt"], Value::Bool(true));
    assert_eq!(
        capabilities["privacyUsageDescriptions"]["NSCameraUsageDescription"],
        "Scan codes."
    );
}

#[test]
fn ipa_analysis_classifies_the_provisioning_profile() {
    let temp = TempDir::new().expect("temp dir");
    let data = analyze(&build_ipa(temp.path()));
    let profile = &data["provisioningProfile"];

    assert_eq!(profile["name"], "Fixture Ad Hoc");
    assert_eq!(profile["teamIdentifier"], "ABCDE12345");
    // Devices listed and debugging disabled is exactly an ad-hoc build.
    assert_eq!(profile["distributionType"], "ad-hoc");
    assert_eq!(profile["provisionedDeviceCount"], 3);
    assert_eq!(profile["expired"], Value::Bool(false));
    assert_eq!(
        profile["entitlements"]["aps-environment"], "production",
        "entitlements come from the profile the app was signed with"
    );
}

#[test]
fn ipa_analysis_rejects_an_archive_without_a_bundle() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("Empty.ipa");
    write_zip(&path, &[("readme.txt", b"not an app".to_vec())]);

    let error = IOSIpaAnalyzer::new()
        .analyze(AnalyzeConfig::new(path.to_string_lossy().into_owned()))
        .expect_err("expected an archive without a payload to be rejected");

    assert!(
        error.to_string().contains("ipa"),
        "error should name the format, got: {error}"
    );
}

// ── Fixture ───────────────────────────────────────────────────────────────────

fn analyze(path: &Path) -> Value {
    IOSIpaAnalyzer::new()
        .analyze(AnalyzeConfig::new(path.to_string_lossy().into_owned()))
        .expect("ipa analysis should succeed")
        .data
}

/// Builds an IPA holding a Flutter app with one plugin, a crash-reporting SDK,
/// a widget extension and an ad-hoc provisioning profile.
fn build_ipa(parent: &Path) -> PathBuf {
    let path = parent.join("Fixture.ipa");
    let app = "Payload/Fixture.app/";
    let binary = thin_mach_o(
        CPU_TYPE_ARM64,
        PLATFORM_IOS,
        &[
            "/System/Library/Frameworks/UIKit.framework/UIKit",
            "/System/Library/Frameworks/SwiftUI.framework/SwiftUI",
            "/usr/lib/libobjc.A.dylib",
            "/usr/lib/swift/libswiftCore.dylib",
            "/usr/lib/libc++.1.dylib",
            "@rpath/Flutter.framework/Flutter",
            "@rpath/url_launcher_ios.framework/url_launcher_ios",
            "@rpath/Sentry.framework/Sentry",
        ],
    );

    let mut entries: Vec<(String, Vec<u8>)> = vec![
        (format!("{app}Info.plist"), INFO_PLIST.as_bytes().to_vec()),
        (format!("{app}Fixture"), binary),
        (
            format!("{app}embedded.mobileprovision"),
            PROVISIONING_PROFILE.as_bytes().to_vec(),
        ),
        (
            format!("{app}_CodeSignature/CodeResources"),
            b"<plist/>".to_vec(),
        ),
        (format!("{app}Assets.car"), vec![0u8; 4096]),
        (
            format!("{app}flutter_assets/AssetManifest.bin"),
            vec![0u8; 128],
        ),
        (
            format!("{app}PlugIns/Widget.appex/Info.plist"),
            EXTENSION_PLIST.as_bytes().to_vec(),
        ),
    ];
    for framework in ["Flutter", "Sentry", "url_launcher_ios"] {
        entries.push((
            format!("{app}Frameworks/{framework}.framework/{framework}"),
            vec![0u8; 2048],
        ));
    }

    let entries: Vec<(&str, Vec<u8>)> = entries
        .iter()
        .map(|(name, contents)| (name.as_str(), contents.clone()))
        .collect();
    write_zip(&path, &entries);
    path
}

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>dev.fastforge.fixture</string>
  <key>CFBundleName</key><string>Fixture</string>
  <key>CFBundleDisplayName</key><string>Fixture App</string>
  <key>CFBundleShortVersionString</key><string>3.1.0</string>
  <key>CFBundleVersion</key><string>77</string>
  <key>CFBundleExecutable</key><string>Fixture</string>
  <key>MinimumOSVersion</key><string>15.0</string>
  <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
  <key>CFBundleSupportedPlatforms</key><array><string>iPhoneOS</string></array>
  <key>DTSDKName</key><string>iphoneos17.2</string>
  <key>DTXcodeBuild</key><string>15C500b</string>
  <key>UIBackgroundModes</key><array><string>remote-notification</string><string>audio</string></array>
  <key>NSCameraUsageDescription</key><string>Scan codes.</string>
  <key>ITSAppUsesNonExemptEncryption</key><false/>
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleURLName</key><string>dev.fastforge.url</string>
      <key>CFBundleURLSchemes</key><array><string>fastforge</string></array>
    </dict>
  </array>
  <key>NSAppTransportSecurity</key><dict><key>NSAllowsArbitraryLoads</key><true/></dict>
</dict>
</plist>
"#;

/// A real profile is CMS-signed; the analyzer digs the plist out of the
/// wrapper, which the surrounding bytes here stand in for.
const PROVISIONING_PROFILE: &str = r#"cms-signature-header
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Name</key><string>Fixture Ad Hoc</string>
  <key>TeamName</key><string>FastForge Ltd</string>
  <key>TeamIdentifier</key><array><string>ABCDE12345</string></array>
  <key>AppIDName</key><string>Fixture</string>
  <key>UUID</key><string>1f0e3dad-9999-4000-8000-99a97a1a1a1a</string>
  <key>Platform</key><array><string>iOS</string></array>
  <key>CreationDate</key><date>2026-01-02T03:04:05Z</date>
  <key>ExpirationDate</key><date>2099-01-02T03:04:05Z</date>
  <key>ProvisionedDevices</key><array><string>d1</string><string>d2</string><string>d3</string></array>
  <key>Entitlements</key>
  <dict>
    <key>get-task-allow</key><false/>
    <key>aps-environment</key><string>production</string>
  </dict>
</dict>
</plist>
cms-signature-trailer"#;

const EXTENSION_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>dev.fastforge.fixture.widget</string>
  <key>CFBundleShortVersionString</key><string>3.1.0</string>
  <key>NSExtension</key>
  <dict><key>NSExtensionPointIdentifier</key><string>com.apple.widgetkit-extension</string></dict>
</dict>
</plist>
"#;
