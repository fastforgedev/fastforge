mod support;

use std::fs;
use support::run_fastforge;

#[test]
fn packages_a_native_directory_without_project_detection() {
    let project = tempfile::tempdir().unwrap();
    let bundle = project.path().join("Swift app");
    fs::create_dir(&bundle).unwrap();
    fs::write(bundle.join("MyApp.exe"), b"application").unwrap();
    fs::write(bundle.join("runtime.dll"), b"runtime").unwrap();

    let run = run_fastforge(
        project.path(),
        &[
            "package",
            "--platform",
            "windows",
            "--targets",
            "zip,tar.gz",
            "--input-dir",
            "Swift app",
            "--app-name",
            "my-app",
            "--app-binary",
            "MyApp",
            "--app-version",
            "1.2.3+42",
        ],
    );
    assert!(run.success, "{}\n{}", run.stdout, run.stderr);
    for suffix in ["zip", "tar.gz"] {
        let artifact = project
            .path()
            .join(format!("dist/1.2.3+42/my-app-1.2.3+42-windows.{suffix}"));
        assert!(artifact.is_file());
    }
}

#[test]
fn workflow_packages_a_native_directory() {
    let project = tempfile::tempdir().unwrap();
    let bundle = project.path().join("native");
    fs::create_dir(&bundle).unwrap();
    fs::write(bundle.join("MyApp"), b"application").unwrap();
    fs::write(
        project.path().join("package.yml"),
        r#"name: Package test
on: workflow_dispatch
jobs:
  package:
    steps:
      - uses: fastforge/package
        with:
          platform: linux
          target: zip
          input-dir: native
          app-name: my-app
          app-binary: MyApp
          app-version: 1.2.3+42
"#,
    )
    .unwrap();

    let run = run_fastforge(
        project.path(),
        &["workflow", "run", "--file", "package.yml"],
    );
    assert!(run.success, "{}\n{}", run.stdout, run.stderr);
    assert!(
        project
            .path()
            .join("dist/1.2.3+42/my-app-1.2.3+42-linux.zip")
            .is_file()
    );
}

#[test]
fn rejects_output_nested_in_the_application_directory() {
    let project = tempfile::tempdir().unwrap();
    let bundle = project.path().join("native");
    fs::create_dir(&bundle).unwrap();
    fs::write(bundle.join("MyApp"), b"application").unwrap();

    let run = run_fastforge(
        project.path(),
        &[
            "package",
            "--platform",
            "linux",
            "--targets",
            "zip",
            "--input-dir",
            "native",
            "--app-name",
            "my-app",
            "--app-binary",
            "MyApp",
            "--app-version",
            "1.2.3",
            "--output",
            "native/nested/dist",
        ],
    );
    assert!(!run.success);
    assert!(run.stderr.contains("outside --input-dir"), "{}", run.stderr);
    assert!(!bundle.join("nested").exists());
}
