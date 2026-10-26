//! Real Flutter → package → launch → upgrade checks. Run explicitly on the
//! matching desktop OS; see README.md for tools and the optional PKG installer.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use fastforge_app_builder::{BuildResult, FlutterAppBuilder};
use fastforge_app_packager::{AppPackager, PackageConfig};
use fastforge_core::Platform;
use serde_json::json;

struct Project {
    _temp: tempfile::TempDir,
    previous: PathBuf,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/flutter_app");
        copy_fixture(&fixture, &root);
        fs::create_dir_all(root.join(".fastforge")).unwrap();
        fs::write(
            root.join(".fastforge/config.yaml"),
            "project:\n  app_id: dev.fastforge.desktopSmoke\n  icon: smoke.png\n",
        )
        .unwrap();
        image::RgbaImage::from_pixel(256, 256, image::Rgba([80, 120, 200, 255]))
            .save(root.join("smoke.png"))
            .unwrap();
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        Self {
            _temp: temp,
            previous,
            root,
        }
    }

    fn build(&self, platform: &Platform, version: &str, number: &str) -> BuildResult {
        FlutterAppBuilder::default()
            .build(
                platform,
                None,
                json!({
                    "target": "lib/desktop_smoke.dart",
                    "build-name": version,
                    "build-number": number,
                    "dart-define": {"FASTFORGE_SMOKE_VERSION": version},
                })
                .as_object()
                .unwrap()
                .clone(),
                None,
            )
            .expect("real Flutter desktop build failed")
    }

    fn package(&self, build: &BuildResult, packager: &dyn AppPackager, version: &str) -> PathBuf {
        let result = packager
            .package(&PackageConfig {
                app_name: "flutter_app".into(),
                app_binary_name: "flutter_app".into(),
                app_version: version.into(),
                build_mode: "release".into(),
                platform: build.platform,
                flavor: None,
                channel: None,
                artifact_name: None,
                package_format: packager.package_format().into(),
                is_installer: packager.name() == "pkg",
                build_output_dir: std::path::absolute(&build.output_directory).unwrap(),
                build_output_files: build
                    .output_files
                    .iter()
                    .map(|p| std::path::absolute(p).unwrap())
                    .collect(),
                output_dir: self.root.join("dist"),
                packaging_root: None,
                environment: Default::default(),
            })
            .expect("native packager failed");
        assert_eq!(result.artifacts.len(), 1);
        let artifact = result.artifacts.into_iter().next().unwrap();
        assert!(artifact.is_file());
        artifact
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.previous).unwrap();
    }
}

fn copy_fixture(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if matches!(
            name.to_str(),
            Some("build" | ".dart_tool" | "ephemeral" | "Pods" | ".symlinks" | "pubspec.lock")
        ) {
            continue;
        }
        let target = destination.join(name);
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &target);
        } else if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn run(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A first-frame marker proves that the packaged engine, AOT and assets loaded.
/// Requiring the newly compiled version also rejects a stale upgrade payload.
fn launch(command: &mut Command, version: &str) {
    let log = tempfile::NamedTempFile::new().unwrap();
    let stdout = log.reopen().unwrap();
    let stderr = stdout.try_clone().unwrap();
    let mut child = match command
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let program = Path::new(command.get_program());
            let listing = Command::new("ls")
                .arg("-le")
                .arg(program)
                .arg(program.parent().unwrap_or(program))
                .output()
                .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
                .unwrap_or_default();
            panic!("cannot launch {}: {error}\n{listing}", program.display());
        }
    };
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "desktop launch timed out: {command:?}\n{}",
                fs::read_to_string(log.path()).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let output = fs::read_to_string(log.path()).unwrap();
    assert!(status.success(), "{command:?}: {status}\n{output}");
    assert!(
        output
            .lines()
            .any(|line| line.trim() == format!("fastforge-smoke:{version}")),
        "first frame/version marker missing: {output}"
    );
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires Flutter Linux toolchain, appimagetool and a graphical session (or Xvfb)"]
fn appimage_flutter_launches_and_upgrades() {
    use fastforge_app_packager::LinuxAppImagePackager;
    let project = Project::new();
    let installed = project.root.join("installed.AppImage");
    for (version, number) in [("1.0.0", "1"), ("1.1.0", "2")] {
        let build = project.build(&Platform::Linux, version, number);
        assert!(build.output_files.is_empty());
        let artifact = project.package(&build, &LinuxAppImagePackager, version);
        let extracted = project.root.join(format!("extracted-{version}"));
        fs::create_dir(&extracted).unwrap();
        run(Command::new(&artifact)
            .arg("--appimage-extract")
            .current_dir(&extracted));
        let appdir = extracted.join("squashfs-root");
        for file in [
            "AppRun",
            "flutter_app",
            "lib/libapp.so",
            "lib/libflutter_linux_gtk.so",
            "data/icudtl.dat",
            "data/flutter_assets/AssetManifest.bin",
            "dev.fastforge.desktopSmoke.desktop",
            "dev.fastforge.desktopSmoke.png",
        ] {
            assert!(
                appdir.join(file).is_file(),
                "missing AppImage payload {file}"
            );
        }
        // AppImages upgrade by replacing the executable at the same path.
        let replacement = project.root.join("replacement.AppImage");
        fs::copy(&artifact, &replacement).unwrap();
        fs::rename(replacement, &installed).unwrap();
        launch(
            Command::new(&installed).arg("--appimage-extract-and-run"),
            version,
        );
    }
    fs::remove_file(installed).unwrap();
}

#[cfg(target_os = "macos")]
struct MountedImage {
    mount: PathBuf,
}

#[cfg(target_os = "macos")]
impl MountedImage {
    fn attach(image: &Path, mount: PathBuf) -> Self {
        fs::create_dir_all(&mount).unwrap();
        // With ownership ignored (the default for disk images), files that
        // root installs are not usable as their recorded owners.
        run(Command::new("hdiutil")
            .args([
                "attach",
                "-owners",
                "on",
                "-nobrowse",
                "-noautoopen",
                "-mountpoint",
            ])
            .arg(&mount)
            .arg(image));
        Self { mount }
    }
}

#[cfg(target_os = "macos")]
impl Drop for MountedImage {
    fn drop(&mut self) {
        let status = Command::new("hdiutil")
            .arg("detach")
            .arg(&self.mount)
            .status();
        if !status.is_ok_and(|status| status.success()) {
            let _ = Command::new("hdiutil")
                .args(["detach", "-force"])
                .arg(&self.mount)
                .status();
        }
    }
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "requires macOS with Flutter, Xcode and a graphical session; optional isolated PKG install uses sudo"]
fn macos_flutter_packages_launch_and_upgrade() {
    use fastforge_app_packager::{MacOSDmgPackager, MacOSPkgPackager};
    let project = Project::new();
    let apps = project.root.join("Applications");
    fs::create_dir(&apps).unwrap();
    let installed = apps.join("flutter_app.app");
    let install_pkg = std::env::var_os("FASTFORGE_TEST_PKG_INSTALL").is_some();
    let volume = if install_pkg {
        // A separate disk image isolates both /Applications and pkg receipts.
        run(Command::new("sudo").args(["-n", "true"]));
        let image = project.root.join("installation.dmg");
        run(Command::new("hdiutil")
            .args([
                "create",
                "-size",
                "1g",
                "-fs",
                "HFS+",
                "-volname",
                "FastforgeTest",
            ])
            .arg(&image));
        Some(MountedImage::attach(
            &image,
            project.root.join("install-volume"),
        ))
    } else {
        eprintln!(
            "PKG payload launch only; set FASTFORGE_TEST_PKG_INSTALL=1 for installer upgrade + receipt checks"
        );
        None
    };
    for (version, number) in [("1.0.0", "1"), ("1.1.0", "2")] {
        let build = project.build(&Platform::MacOS, version, number);
        assert_eq!(build.output_files.len(), 1, "expected one .app");
        let dmg = project.package(&build, &MacOSDmgPackager, version);
        {
            let image = MountedImage::attach(&dmg, project.root.join("dmg-volume"));
            let bundle = image.mount.join("flutter_app.app");
            assert_eq!(
                fs::read_link(image.mount.join("Applications")).unwrap(),
                Path::new("/Applications")
            );
            launch(
                &mut Command::new(bundle.join("Contents/MacOS/flutter_app")),
                version,
            );
            if installed.exists() {
                fs::remove_dir_all(&installed).unwrap();
            }
            run(Command::new("ditto").arg(&bundle).arg(&installed));
        }
        launch(
            &mut Command::new(installed.join("Contents/MacOS/flutter_app")),
            version,
        );

        let pkg = project.package(&build, &MacOSPkgPackager::default(), version);
        let expanded = project.root.join(format!("pkg-{version}"));
        run(Command::new("pkgutil")
            .arg("--expand-full")
            .arg(&pkg)
            .arg(&expanded));
        let distribution = fs::read_to_string(expanded.join("Distribution")).unwrap();
        assert!(distribution.contains("enable_localSystem=\"true\""));
        let component = fs::read_dir(&expanded)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|p| p.is_dir() && p.extension().is_some_and(|ext| ext == "pkg"))
            .expect("component package missing");
        let info = fs::read_to_string(component.join("PackageInfo")).unwrap();
        for removed in [
            "<relocate>",
            "<upgrade-bundle>",
            "<update-bundle",
            "<atomic-update-bundle",
            "<strict-identifier>",
        ] {
            assert!(
                !info.contains(removed),
                "relocation metadata left in PKG: {info}"
            );
        }
        let payload = component.join("Payload/flutter_app.app");
        launch(
            &mut Command::new(payload.join("Contents/MacOS/flutter_app")),
            version,
        );
        if let Some(volume) = &volume {
            // The package installs on the startup disk only, and its
            // volume check requires macOS on the target; a copy without
            // either exercises the same payload, upgrade and receipt on the
            // isolated volume.
            let editable = project.root.join(format!("anywhere-{version}"));
            run(Command::new("pkgutil")
                .arg("--expand")
                .arg(&pkg)
                .arg(&editable));
            let distribution_path = editable.join("Distribution");
            let mut distribution = fs::read_to_string(&distribution_path)
                .unwrap()
                .replace("enable_anywhere=\"false\"", "enable_anywhere=\"true\"");
            if let (Some(start), Some(end)) = (
                distribution.find("<volume-check>"),
                distribution.find("</volume-check>"),
            ) {
                distribution.replace_range(start..end + "</volume-check>".len(), "");
            }
            fs::write(&distribution_path, distribution).unwrap();
            let anywhere = project.root.join(format!("anywhere-{version}.pkg"));
            run(Command::new("pkgutil")
                .arg("--flatten")
                .arg(&editable)
                .arg(&anywhere));
            run(Command::new("sudo")
                .args(["-n", "installer", "-pkg"])
                .arg(&anywhere)
                .arg("-target")
                .arg(&volume.mount));
            let bundle = volume.mount.join("Applications/flutter_app.app");
            launch(
                &mut Command::new(bundle.join("Contents/MacOS/flutter_app")),
                version,
            );
            let plist_version = run(Command::new("/usr/libexec/PlistBuddy")
                .args(["-c", "Print :CFBundleShortVersionString"])
                .arg(bundle.join("Contents/Info.plist")));
            assert_eq!(plist_version.trim(), version);
            let receipts = run(Command::new("pkgutil")
                .arg("--volume")
                .arg(&volume.mount)
                .arg("--pkgs"));
            assert!(
                receipts.contains("dev.fastforge.flutterApp"),
                "missing isolated install receipt: {receipts}"
            );
        }
    }
    fs::remove_dir_all(installed).unwrap();
}
