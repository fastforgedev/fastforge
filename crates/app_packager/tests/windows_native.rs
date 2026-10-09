//! Native Windows-tool tests. Run on Windows with Inno Setup 6 and Windows
//! SDK makeappx/signtool on PATH:
//! cargo test -p fastforge_app_packager --test windows_native -- --ignored --nocapture
//!
//! Uses a real Windows executable as the bundle. Installs and uninstalls the
//! generated EXE in a temporary directory; raw EXE is compiled only. MSIX is
//! unpacked and inspected. Optional MSIX_CERTIFICATE_PATH/PASSWORD and
//! MSIX_PUBLISHER enable signing without installing a certificate.
#![cfg(target_os = "windows")]

use std::path::{Path, PathBuf};
use std::process::Command;

use fastforge_app_packager::{AppPackager, PackageConfig, WindowsExePackager, WindowsMsixPackager};
use fastforge_core::Platform;

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn run(command: &mut Command) -> String {
    let output = command.output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "{:?}: {}",
        command.get_program(),
        text
    );
    text
}

struct WorkingDirectory(PathBuf);
impl Drop for WorkingDirectory {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).unwrap();
    }
}

/// Ensure a failed launch/assertion still removes the test installation.
struct InstalledApp(PathBuf);
impl Drop for InstalledApp {
    fn drop(&mut self) {
        let uninstaller = self.0.join("unins000.exe");
        if uninstaller.is_file() {
            let _ = Command::new(uninstaller)
                .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
                .status();
        }
    }
}

#[test]
#[ignore = "installs an EXE in a temporary directory; needs Windows SDK and Inno Setup 6"]
fn native_exe_install_uninstall_and_msix_unpack() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let _cwd = WorkingDirectory(std::env::current_dir().unwrap());
    std::env::set_current_dir(root).unwrap();
    write(
        &root.join("pubspec.yaml"),
        "name: demo\nversion: 1.2.3+4\ndev_dependencies:\n  msix: any\n",
    );
    write(
        &root.join(".fastforge/config.yaml"),
        &format!(
            "project:\n  display_name: Fastforge Packaging Test\n  app_id: dev.fastforge.smoke.{}\n  description: Native Windows packaging test\n  homepage: https://example.com\n",
            std::process::id()
        ),
    );
    write(
        &root.join(".fastforge/packaging/windows/shared/files/version.txt"),
        "${APP_VERSION}",
    );
    let bundle = root.join("build/windows/x64/runner/Release");
    std::fs::create_dir_all(&bundle).unwrap();
    let executable = Path::new(&std::env::var("SystemRoot").unwrap()).join("System32/whoami.exe");
    std::fs::copy(&executable, bundle.join("demo.exe")).unwrap();
    let artifacts = std::env::var_os("FASTFORGE_NATIVE_ARTIFACTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("artifacts"));
    for variant in ["generated", "raw"] {
        if variant == "raw" {
            write(
                &root.join(".fastforge/packaging/windows/exe/setup.iss"),
                include_str!(
                    "../../../examples/hello_world/.fastforge/packaging/windows/exe/setup.iss"
                ),
            );
            write(
                &root.join(".fastforge/packaging/windows/msix/AppxManifest.xml"),
                include_str!(
                    "../../../examples/hello_world/.fastforge/packaging/windows/msix/AppxManifest.xml"
                ),
            );
        }
        let mut config = PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "demo".into(),
            app_version: "1.2.3+4".into(),
            build_mode: "release".into(),
            platform: Platform::Windows,
            flavor: None,
            channel: None,
            artifact_name: Some(format!("{variant}.{{{{ext}}}}")),
            package_format: "exe".into(),
            is_installer: true,
            build_output_dir: bundle.clone(),
            build_output_files: vec![],
            output_dir: artifacts.clone(),
            environment: Default::default(),
        };
        let exe = WindowsExePackager
            .package(&config)
            .unwrap()
            .artifacts
            .remove(0);
        assert!(std::fs::metadata(&exe).unwrap().len() > 1024);
        println!("EXE {variant}: {}", exe.display());
        if variant == "generated" {
            let install = root.join("installed");
            let _installed = InstalledApp(install.clone());
            run(Command::new(&exe)
                .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"])
                .arg(format!("/DIR={}", install.display())));
            assert_eq!(
                std::fs::read(install.join("demo.exe")).unwrap(),
                std::fs::read(&executable).unwrap()
            );
            assert_eq!(
                std::fs::read_to_string(install.join("version.txt")).unwrap(),
                "1.2.3+4"
            );
            run(Command::new(install.join("demo.exe")).arg("/user"));
            run(Command::new(install.join("unins000.exe")).args([
                "/VERYSILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
            ]));
            for _ in 0..100 {
                if !install.join("demo.exe").exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            assert!(
                !install.join("demo.exe").exists(),
                "uninstaller left the application installed"
            );
            println!("EXE install, launch and uninstall passed");
        }
        config.package_format = "msix".into();
        config.is_installer = false;
        let msix = WindowsMsixPackager::default()
            .package(&config)
            .unwrap()
            .artifacts
            .remove(0);
        let unpack = root.join(format!("unpacked-{variant}"));
        run(Command::new("makeappx")
            .arg("unpack")
            .arg("/p")
            .arg(&msix)
            .arg("/d")
            .arg(&unpack)
            .arg("/o"));
        assert_eq!(
            std::fs::read(unpack.join("demo.exe")).unwrap(),
            std::fs::read(&executable).unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(unpack.join("version.txt")).unwrap(),
            "1.2.3+4"
        );
        assert!(unpack.join("AppxManifest.xml").is_file());
        for (name, dimensions) in [
            ("StoreLogo.png", (50, 50)),
            ("Square150x150Logo.png", (150, 150)),
            ("Square44x44Logo.png", (44, 44)),
        ] {
            let icon = image::open(unpack.join("Assets").join(name)).unwrap();
            assert_eq!((icon.width(), icon.height()), dimensions);
        }
        if std::env::var_os("MSIX_CERTIFICATE_PATH").is_some() {
            assert!(unpack.join("AppxSignature.p7x").is_file());
        }
        println!(
            "MSIX {variant} pack, sign (when configured) and unpack: {}",
            msix.display()
        );
    }
}
