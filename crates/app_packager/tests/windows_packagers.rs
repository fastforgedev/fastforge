//! Tool-contract tests with executable stand-ins, runnable on Unix hosts.
//! These verify orchestration; they do not validate an installer with Windows.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use fastforge_app_packager::{AppPackager, PackageConfig, WindowsExePackager, WindowsMsixPackager};
use fastforge_core::Platform;

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn tool(path: &Path, script: &str) {
    write(path, script);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn config(root: &Path, format: &str) -> PackageConfig {
    PackageConfig {
        app_name: "demo".into(),
        app_binary_name: "runner".into(),
        app_version: "1.2.3+4".into(),
        build_mode: "release".into(),
        platform: Platform::Windows,
        flavor: None,
        channel: None,
        artifact_name: Some("nested/{{name}}.{{ext}}".into()),
        package_format: format.into(),
        is_installer: format == "exe",
        build_output_dir: root.join("build/windows/arm64/runner/Release"),
        build_output_files: vec![],
        output_dir: root.join("dist"),
        environment: [
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", root.join("tools").display()),
            ),
            (
                "INNO_SETUP_PATH".into(),
                root.join("tools").display().to_string(),
            ),
        ]
        .into(),
    }
}

/// Restore the working directory even if an assertion fails.
struct WorkingDirectory(PathBuf);
impl Drop for WorkingDirectory {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).unwrap();
    }
}

#[test]
fn native_tools_receive_staged_application_artifact_paths_and_project_environment() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let _cwd = WorkingDirectory(std::env::current_dir().unwrap());
    std::env::set_current_dir(root).unwrap();
    write(
        &root.join("pubspec.yaml"),
        "name: demo\ndev_dependencies:\n  msix: any\nmsix_config:\n  sign_msix: false\n",
    );
    write(
        &root.join(".fastforge/config.yaml"),
        "project:\n  display_name: Demo App\n  app_id: dev.example.demo\nenv:\n  CONTRACT_VALUE: from-project\n",
    );
    write(
        &root.join("build/windows/arm64/runner/Release/runner.exe"),
        "application",
    );
    write(
        &root.join(".fastforge/packaging/windows/shared/files/version.txt"),
        "${APP_VERSION}",
    );
    write(
        &root.join(".fastforge/packaging/windows/exe/setup.iss"),
        "[Setup]\nAppName=${APP_DISPLAY_NAME}\nAppId=${APP_ID}\n[Files]\nSource: \"${PACKAGING_DIRECTORY}\\*\"; DestDir: \"{app}\"\n",
    );
    tool(
        &root.join("tools/ISCC.exe"),
        r#"#!/bin/sh
set -eu
test "$CONTRACT_VALUE" = from-project
test "$PACKAGE_ARCH" = arm64
test -f "$PACKAGING_DIRECTORY/runner.exe"
test "$(cat "$PACKAGING_DIRECTORY/version.txt")" = 1.2.3+4
outdir=""; outname=""; script=""
for arg in "$@"; do
    case "$arg" in /O*) outdir="${arg#/O}";; /F*) outname="${arg#/F}";; *) script="$arg";; esac
done
test "$outdir/$outname.exe" = "$OUTPUT_ARTIFACT_PATH"
mkdir -p "$outdir"
cp "$script" "$OUTPUT_ARTIFACT_PATH"
"#,
    );
    tool(
        &root.join("tools/makeappx"),
        r#"#!/bin/sh
set -eu
test "$CONTRACT_VALUE" = from-project
test "$PACKAGE_ARCH" = arm64
test -f "$PACKAGING_DIRECTORY/runner.exe"
test -f "$PACKAGING_DIRECTORY/Assets/Square44x44Logo.png"
test "$(cat "$PACKAGING_DIRECTORY/version.txt")" = 1.2.3+4
test "$1" = pack
test "$2" = /d
test "$3" = "$PACKAGING_DIRECTORY"
test "$4" = /p
test "$5" = "$OUTPUT_ARTIFACT_PATH"
test "$6" = /o
test "$#" = 6
mkdir -p "$(dirname "$OUTPUT_ARTIFACT_PATH")"
cp "$PACKAGING_DIRECTORY/AppxManifest.xml" "$OUTPUT_ARTIFACT_PATH"
"#,
    );
    tool(
        &root.join("tools/dart"),
        "#!/bin/sh\ntouch dart-was-called\nexit 1\n",
    );
    let exe = WindowsExePackager
        .package(&config(root, "exe"))
        .unwrap()
        .artifacts
        .remove(0);
    let script = std::fs::read_to_string(exe).unwrap();
    assert!(script.contains("AppName=Demo App\nAppId=dev.example.demo"));
    let msix = WindowsMsixPackager::default()
        .package(&config(root, "msix"))
        .unwrap()
        .artifacts
        .remove(0);
    let manifest = std::fs::read_to_string(msix).unwrap();
    assert!(manifest.contains("ProcessorArchitecture=\"arm64\""));
    assert!(manifest.contains("<DisplayName>Demo App</DisplayName>"));
    assert!(!root.join("dart-was-called").exists());
    assert!(!root.join("dist/1.2.3+4/nested/demo_exe").exists());
    assert!(!root.join("dist/1.2.3+4/nested/demo_msix").exists());

    // Native signing keeps paths and passwords with spaces as single args,
    // and receives the same configured environment as makeappx.
    write(
        &root.join("pubspec.yaml"),
        "name: demo\ndev_dependencies:\n  msix: any\n",
    );
    write(
        &root.join("certificate with spaces.pfx"),
        "test certificate",
    );
    tool(
        &root.join("tools/signtool"),
        r#"#!/bin/sh
set -eu
test "$#" = 9
test "$1" = sign
test "$2" = /fd
test "$3" = SHA256
test "$4" = /a
test "$5" = /f
test "$6" = "$MSIX_CERTIFICATE_PATH"
test "$7" = /p
test "$8" = "$MSIX_CERTIFICATE_PASSWORD"
test "$9" = "$OUTPUT_ARTIFACT_PATH"
test "$CONTRACT_VALUE" = from-project
touch "$OUTPUT_DIRECTORY/signed.txt"
"#,
    );
    let mut signed = config(root, "msix");
    signed.environment.insert(
        "MSIX_CERTIFICATE_PATH".into(),
        root.join("certificate with spaces.pfx")
            .display()
            .to_string(),
    );
    signed.environment.insert(
        "MSIX_CERTIFICATE_PASSWORD".into(),
        "password with spaces".into(),
    );
    WindowsMsixPackager::default().package(&signed).unwrap();
    assert!(root.join("dist/signed.txt").is_file());
}
