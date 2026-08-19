//! Covers the detail the APK and AAB analyzers extract. Both need Android SDK
//! tools to read a package's identity, so the fixtures ship stub `aapt2`,
//! `apksigner` and `bundletool` executables that replay recorded output — the
//! analyzers' own parsing is what is under test.

mod support;

use fastforge_app_analyzer::{AnalyzeConfig, AndroidAabAnalyzer, AndroidApkAnalyzer, AppAnalyzer};
use serde_json::Value;
use serial_test::serial;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use support::write_zip;
use tempfile::TempDir;

#[test]
#[serial]
fn apk_analysis_reports_identity_and_manifest() {
    let temp = TempDir::new().expect("temp dir");
    let apk = build_apk(temp.path());
    let sdk = build_fake_sdk(temp.path());

    let data = with_env(&[("ANDROID_HOME", &sdk.to_string_lossy())], || {
        analyze_apk(&apk)
    });

    assert_eq!(data["platform"], "android");
    assert_eq!(data["format"], "apk");
    assert_eq!(data["identifier"], "dev.fastforge.fixture");
    assert_eq!(data["name"], "Fixture App");
    assert_eq!(data["version"], "2.1.0");
    // Android version codes are integers, unlike Apple's build strings.
    assert_eq!(data["buildNumber"], 42);
    assert_eq!(data["fileName"], "app-release.apk");
    assert_eq!(data["sha256"].as_str().unwrap().len(), 64);

    let manifest = &data["manifest"];
    assert_eq!(manifest["minSdkVersion"], 24);
    assert_eq!(manifest["targetSdkVersion"], 35);
    assert_eq!(manifest["compileSdkVersion"], 35);
    assert_eq!(
        manifest["launchableActivity"],
        "dev.fastforge.fixture.MainActivity"
    );
    assert_eq!(
        manifest["permissions"],
        Value::Array(vec![
            "android.permission.INTERNET".into(),
            "android.permission.CAMERA".into()
        ])
    );
    assert_eq!(
        manifest["locales"],
        Value::Array(vec!["en".into(), "zh-CN".into()]),
        "the unqualified `--_--` locale is aapt2 bookkeeping, not a language"
    );
    assert_eq!(
        data["abis"],
        Value::Array(vec!["arm64-v8a".into(), "armeabi-v7a".into()])
    );
}

#[test]
#[serial]
fn apk_analysis_reads_the_tech_stack_from_the_payload() {
    let temp = TempDir::new().expect("temp dir");
    let apk = build_apk(temp.path());
    let sdk = build_fake_sdk(temp.path());

    let data = with_env(&[("ANDROID_HOME", &sdk.to_string_lossy())], || {
        analyze_apk(&apk)
    });
    let stack = &data["techStack"];

    assert_eq!(stack["runtime"], "flutter");
    assert_eq!(
        stack["flutter"]["aot"],
        Value::Bool(true),
        "a release build compiles Dart into libapp.so"
    );
    assert_eq!(
        stack["languages"],
        Value::Array(vec!["Kotlin".into(), "Dart".into(), "C/C++".into()])
    );
    assert_eq!(
        stack["uiToolkits"],
        Value::Array(vec!["Jetpack Compose".into()])
    );
    assert_eq!(stack["buildTools"]["androidGradlePlugin"], "8.7.2");
    assert_eq!(stack["buildTools"]["gradle"], "8.9");
    assert_eq!(stack["buildTools"]["kotlin"], "2.1.0");
    assert_eq!(stack["buildTools"]["javaTarget"], "17");
    assert_eq!(
        stack["nativeLibraries"],
        Value::Array(vec!["libapp.so".into(), "libflutter.so".into()])
    );

    let libraries = stack["libraries"].as_array().unwrap();
    assert_eq!(libraries[0]["name"], "androidx.compose.ui:ui");
    assert_eq!(libraries[0]["version"], "1.8.0");
    assert_eq!(
        libraries.len(),
        2,
        "a marker holding an unresolved Gradle placeholder is not a version: {libraries:?}"
    );

    assert_eq!(data["contents"]["dexCount"], 2);
    assert!(data["contents"]["sizeBreakdown"]["lib"].as_u64().unwrap() > 0);
}

#[test]
#[serial]
fn apk_analysis_reports_the_signing_certificate() {
    let temp = TempDir::new().expect("temp dir");
    let apk = build_apk(temp.path());
    let sdk = build_fake_sdk(temp.path());

    let data = with_env(&[("ANDROID_HOME", &sdk.to_string_lossy())], || {
        analyze_apk(&apk)
    });
    let signature = &data["signature"];

    assert_eq!(signature["verified"], Value::Bool(true));
    assert_eq!(
        signature["schemes"],
        Value::Array(vec!["v2".into(), "v3".into()]),
        "only the schemes that verified should be listed"
    );
    let signer = &signature["signers"][0];
    assert_eq!(signer["subject"], "CN=FastForge, O=FastForge, C=CN");
    assert_eq!(
        signer["sha256"],
        "6f35ec5b46b8cbc43a4a6b6e5e5e1908eed6707006a538c76e42ce663f543d7c"
    );
    assert_eq!(signer["keyAlgorithm"], "RSA");
    assert_eq!(signer["keySizeBits"], 2048);
    assert_eq!(
        signature["signers"].as_array().unwrap().len(),
        1,
        "one certificate listed under two schemes is still one signer"
    );
}

#[test]
#[serial]
fn aab_analysis_reports_modules_and_dependencies() {
    let temp = TempDir::new().expect("temp dir");
    let aab = build_aab(temp.path());
    let bundletool = write_stub(
        &temp.path().join("tools"),
        "bundletool",
        &format!("cat <<'XML'\n{}\nXML", MANIFEST_XML),
    );
    // An SDK without build-tools forces the bundletool fallback path.
    let empty_sdk = temp.path().join("empty-sdk");
    std::fs::create_dir_all(&empty_sdk).expect("create sdk dir");

    let data = with_env(
        &[
            ("ANDROID_HOME", &empty_sdk.to_string_lossy()),
            ("BUNDLETOOL", &bundletool.to_string_lossy()),
        ],
        || {
            AndroidAabAnalyzer::new()
                .analyze(AnalyzeConfig::new(aab.to_string_lossy().into_owned()))
                .expect("aab analysis should succeed")
                .data
        },
    );

    assert_eq!(data["format"], "aab");
    assert_eq!(data["identifier"], "dev.fastforge.fixture");
    assert_eq!(data["version"], "2.1.0");
    assert_eq!(data["buildNumber"], 42);
    assert_eq!(data["manifest"]["minSdkVersion"], 24);
    assert_eq!(
        data["abis"],
        Value::Array(vec!["arm64-v8a".into()]),
        "without aapt2 the ABIs come from the bundle's own lib layout"
    );

    let modules = data["modules"].as_array().unwrap();
    assert_eq!(modules[0]["name"], "base");
    assert_eq!(modules[0]["dexCount"], 1);
    assert_eq!(modules[0]["hasNativeLibraries"], Value::Bool(true));
    assert_eq!(
        modules[1]["name"], "premium",
        "dynamic feature modules should be listed alongside the base module"
    );

    // A bundle records the full dependency graph the build resolved.
    let dependencies = data["techStack"]["dependencies"].as_array().unwrap();
    assert_eq!(dependencies.len(), 2);
    assert_eq!(dependencies[0]["name"], "androidx.core:core");
    assert_eq!(dependencies[0]["version"], "1.17.0");
    assert_eq!(
        dependencies[1]["name"],
        "com.google.android.material:material"
    );

    assert_eq!(data["techStack"]["runtime"], "flutter");
    assert_eq!(data["signature"]["jarSigned"], Value::Bool(false));
}

#[test]
#[serial]
fn apk_analysis_fails_without_the_android_sdk() {
    let temp = TempDir::new().expect("temp dir");
    let apk = build_apk(temp.path());
    let empty_sdk = temp.path().join("empty-sdk");
    std::fs::create_dir_all(&empty_sdk).expect("create sdk dir");

    let error = with_env(&[("ANDROID_HOME", &empty_sdk.to_string_lossy())], || {
        AndroidApkAnalyzer::new()
            .analyze(AnalyzeConfig::new(apk.to_string_lossy().into_owned()))
            .expect_err("expected the missing tool to be reported")
    });

    assert!(
        error.to_string().contains("aapt2"),
        "error should name the missing tool, got: {error}"
    );
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn analyze_apk(path: &Path) -> Value {
    AndroidApkAnalyzer::new()
        .analyze(AnalyzeConfig::new(path.to_string_lossy().into_owned()))
        .expect("apk analysis should succeed")
        .data
}

/// Runs `body` with the given environment variables set.
///
/// The analyzers locate the SDK through the environment, which is process-wide
/// state — every test using this is `#[serial]`.
fn with_env<T>(variables: &[(&str, &str)], body: impl FnOnce() -> T) -> T {
    for (key, value) in variables {
        // SAFETY: `#[serial]` keeps these tests from running concurrently, so
        // nothing else touches the environment while it is being changed.
        unsafe { std::env::set_var(key, value) };
    }
    let result = body();
    for (key, _) in variables {
        // SAFETY: as above.
        unsafe { std::env::remove_var(key) };
    }
    result
}

/// An APK holding a Flutter release build with Compose on the Android side.
fn build_apk(parent: &Path) -> PathBuf {
    let path = parent.join("app-release.apk");
    let mut entries: Vec<(String, Vec<u8>)> = vec![
        ("AndroidManifest.xml".to_string(), vec![3, 0, 8, 0]),
        ("classes.dex".to_string(), vec![0u8; 2048]),
        ("classes2.dex".to_string(), vec![0u8; 1024]),
        ("resources.arsc".to_string(), vec![0u8; 512]),
        (
            "assets/flutter_assets/AssetManifest.bin".to_string(),
            vec![0u8; 128],
        ),
        (
            "assets/flutter_assets/NativeAssetsManifest.json".to_string(),
            b"{}".to_vec(),
        ),
        (
            "kotlin/kotlin.kotlin_builtins".to_string(),
            b"kotlin".to_vec(),
        ),
        (
            "kotlin-tooling-metadata.json".to_string(),
            KOTLIN_METADATA.as_bytes().to_vec(),
        ),
        (
            "META-INF/com/android/build/gradle/app-metadata.properties".to_string(),
            b"appMetadataVersion=1.1\nandroidGradlePluginVersion=8.7.2\n".to_vec(),
        ),
        (
            "META-INF/androidx.compose.ui_ui.version".to_string(),
            b"1.8.0".to_vec(),
        ),
        (
            "META-INF/androidx.core_core.version".to_string(),
            b"1.17.0".to_vec(),
        ),
        (
            // AndroidX occasionally ships a marker that never got resolved.
            "META-INF/androidx.arch.core_core-runtime.version".to_string(),
            b"task ':arch:core:core-runtime:writeVersionFile' property 'version'".to_vec(),
        ),
    ];
    for abi in ["arm64-v8a", "armeabi-v7a"] {
        entries.push((format!("lib/{abi}/libflutter.so"), vec![0u8; 4096]));
        entries.push((format!("lib/{abi}/libapp.so"), vec![0u8; 8192]));
    }

    let entries: Vec<(&str, Vec<u8>)> = entries
        .iter()
        .map(|(name, contents)| (name.as_str(), contents.clone()))
        .collect();
    write_zip(&path, &entries);
    path
}

/// An app bundle with a base module, a dynamic feature and the dependency
/// metadata the Android Gradle plugin records.
fn build_aab(parent: &Path) -> PathBuf {
    let path = parent.join("app-release.aab");
    let entries: Vec<(&str, Vec<u8>)> = vec![
        ("base/manifest/AndroidManifest.xml", vec![3, 0, 8, 0]),
        ("base/dex/classes.dex", vec![0u8; 2048]),
        ("base/lib/arm64-v8a/libflutter.so", vec![0u8; 4096]),
        ("base/lib/arm64-v8a/libapp.so", vec![0u8; 8192]),
        (
            "base/assets/flutter_assets/AssetManifest.bin",
            vec![0u8; 128],
        ),
        (
            "base/root/META-INF/androidx.core_core.version",
            b"1.17.0".to_vec(),
        ),
        (
            "BUNDLE-METADATA/com.android.tools.build.gradle/app-metadata.properties",
            b"androidGradlePluginVersion=8.7.2\n".to_vec(),
        ),
        (
            "BUNDLE-METADATA/com.android.tools.build.libraries/dependencies.pb",
            app_dependencies(&[
                ("androidx.core", "core", "1.17.0"),
                ("com.google.android.material", "material", "1.12.0"),
            ]),
        ),
        ("premium/manifest/AndroidManifest.xml", vec![3, 0, 8, 0]),
        ("premium/dex/classes.dex", vec![0u8; 512]),
    ];
    write_zip(&path, &entries);
    path
}

/// Encodes an `AppDependencies` protobuf: repeated `Library` (field 1), each
/// wrapping a `MavenLibrary` (field 1) of group, artifact and version.
fn app_dependencies(libraries: &[(&str, &str, &str)]) -> Vec<u8> {
    fn varint(mut value: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            bytes.push(if value > 0 { byte | 0x80 } else { byte });
            if value == 0 {
                return bytes;
            }
        }
    }
    fn field(number: u64, payload: &[u8]) -> Vec<u8> {
        let mut bytes = varint(number << 3 | 2);
        bytes.extend(varint(payload.len() as u64));
        bytes.extend_from_slice(payload);
        bytes
    }

    libraries
        .iter()
        .flat_map(|(group, artifact, version)| {
            let mut maven = field(1, group.as_bytes());
            maven.extend(field(2, artifact.as_bytes()));
            maven.extend(field(3, version.as_bytes()));
            field(1, &field(1, &maven))
        })
        .collect()
}

/// A fake SDK whose build-tools replay recorded `aapt2` and `apksigner` output.
fn build_fake_sdk(parent: &Path) -> PathBuf {
    let sdk = parent.join("sdk");
    let build_tools = sdk.join("build-tools").join("36.0.0");
    write_stub(
        &build_tools,
        "aapt2",
        &format!("cat <<'EOF'\n{BADGING}\nEOF"),
    );
    write_stub(
        &build_tools,
        "apksigner",
        &format!("cat <<'EOF'\n{APKSIGNER}\nEOF"),
    );
    sdk
}

fn write_stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("create stub directory");
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write stub");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("make stub executable");
    path
}

const BADGING: &str = r#"package: name='dev.fastforge.fixture' versionCode='42' versionName='2.1.0' platformBuildVersionName='15' compileSdkVersion='35' compileSdkVersionCodename='15'
minSdkVersion:'24'
targetSdkVersion:'35'
uses-permission: name='android.permission.INTERNET'
uses-permission: name='android.permission.CAMERA'
application-label:'Fixture App'
application-label-zh:'固定装置'
application: label='Fixture App' icon='res/icon.png'
launchable-activity: name='dev.fastforge.fixture.MainActivity'  label='' icon=''
uses-feature: name='android.hardware.camera'
uses-implied-feature: name='android.hardware.faketouch' reason='default feature for all apps'
supports-screens: 'small' 'normal' 'large' 'xlarge'
locales: '--_--' 'en' 'zh-CN'
densities: '160' '480'
native-code: 'arm64-v8a' 'armeabi-v7a'"#;

const APKSIGNER: &str = r#"Verifies
Verified using v1 scheme (JAR signing): false
Verified using v2 scheme (APK Signature Scheme v2): true
Verified using v3 scheme (APK Signature Scheme v3): true
Verified using v4 scheme (APK Signature Scheme v4): false
Number of signers: 1
V2 Signer: certificate DN: CN=FastForge, O=FastForge, C=CN
V2 Signer: certificate SHA-256 digest: 6f35ec5b46b8cbc43a4a6b6e5e5e1908eed6707006a538c76e42ce663f543d7c
V2 Signer: key algorithm: RSA
V2 Signer: key size (bits): 2048
V3 Signer: certificate DN: CN=FastForge, O=FastForge, C=CN
V3 Signer: certificate SHA-256 digest: 6f35ec5b46b8cbc43a4a6b6e5e5e1908eed6707006a538c76e42ce663f543d7c
V3 Signer: key algorithm: RSA
V3 Signer: key size (bits): 2048"#;

const MANIFEST_XML: &str = r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="dev.fastforge.fixture" android:versionCode="42" android:versionName="2.1.0">
  <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="35" />
  <uses-permission android:name="android.permission.INTERNET" />
  <application android:label="@string/app_name" />
</manifest>"#;

const KOTLIN_METADATA: &str = r#"{
  "buildSystem": "Gradle",
  "buildSystemVersion": "8.9",
  "buildPluginVersion": "2.1.0",
  "projectTargets": [{"extras": {"android": {"targetCompatibility": "17"}}}]
}"#;
