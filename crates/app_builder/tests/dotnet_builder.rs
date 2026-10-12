//! Real-toolchain integration test for the .NET Builder (native Windows
//! projects, no `pubspec.yaml`). Runs the actual `dotnet publish` against
//! `fixtures/native_windows` and asserts on the publish directory. The
//! argument-encoding and output-path logic is covered by the unit tests in
//! `src/dotnet/mod.rs`; this test adds only the toolchain-dependent coverage.
//!
//! Windows only, with the .NET SDK on `PATH`:
//! cargo test -p fastforge_app_builder --test dotnet_builder -- --ignored --nocapture
#![cfg(target_os = "windows")]

mod support;

use fastforge_app_builder::{DotnetAppBuilder, DotnetProject};
use serde_json::{Map, json};
use serial_test::serial;
use std::path::Path;
use support::{WorkingDir, fixture_dir};

#[test]
#[serial]
#[ignore = "runs dotnet publish; needs the .NET SDK on PATH"]
fn publishes_windows_executable() {
    let _cwd = WorkingDir::enter(&fixture_dir("native_windows"));

    let mut arguments = Map::new();
    arguments.insert("project".to_string(), json!("NativeWindowsFixture.csproj"));
    arguments.insert("configuration".to_string(), json!("Release"));
    arguments.insert("runtime-identifier".to_string(), json!("win-x64"));

    let result = DotnetAppBuilder::default()
        .build("windows", Some("windows-dotnet"), arguments, None)
        .expect("dotnet publish should succeed for the console fixture");

    assert_eq!(
        result.output_directory,
        Path::new("bin/fastforge/Release/x64/publish")
    );
    assert!(
        result
            .output_directory
            .join("NativeWindowsFixture.exe")
            .is_file(),
        "expected the apphost in the publish directory"
    );
    assert!(result.output_files.is_empty());

    let project = DotnetProject::load(Path::new("NativeWindowsFixture.csproj")).unwrap();
    assert_eq!(project.assembly_name, "NativeWindowsFixture");
    assert_eq!(project.version.as_deref(), Some("1.2.3"));
    assert!(project.manifest_path.is_some());
}
