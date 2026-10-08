//! Real-tool tests of the Linux packagers.
//!
//! Each test packages a small fake bundle (an ELF binary and a shared
//! library copied from the host) with the generated defaults plus shared
//! templates, inspects the package with the format's own tools, and, when
//! the host allows unprivileged user namespaces (`unshare -r`), installs,
//! upgrades and removes it in a temporary root to check what the package
//! manager does with it (files, the `/usr/bin` link, configuration files).
//!
//! They are `#[ignore]`d: they need a Linux host with the format's tools
//! (`dpkg-deb`/`dpkg`, `rpmbuild`/`rpm`, `bsdtar`/`pacman`, `appimagetool`).
//! A test whose packaging tool is missing is skipped with a message; install
//! checks are skipped when the package manager or `unshare` is missing.
//!
//! ```sh
//! cargo test -p fastforge_app_packager --test linux_packagers -- --ignored --test-threads=1
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use fastforge_app_packager::{
    AppPackager, LinuxAppImagePackager, LinuxDebPackager, LinuxPacmanPackager, LinuxRpmPackager,
    PackageConfig,
};
use fastforge_core::Platform;

/// The packagers read the project from the working directory.
static CWD: Mutex<()> = Mutex::new(());

const APP_ID: &str = "dev.example.hola";
const CONF: &str = "etc/dev.example.hola/app.conf";

struct Project {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

fn write(path: &Path, content: impl AsRef<[u8]>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

/// The first existing file among `candidates`.
fn host_file(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.is_file())
}

fn project() -> Project {
    let tmp = tempfile::tempdir().unwrap();
    let root = std::path::absolute(tmp.path().join("hola")).unwrap();
    write(
        &root.join(".fastforge/config.yaml"),
        "project:\n  display_name: Hola\n  description: A packaging test app\n  \
         homepage: https://example.com\n  app_id: dev.example.hola\n  icon: icon.png\n  \
         license: MIT\n  maintainer: Test <test@example.com>\n",
    );
    let linux = root.join(".fastforge/packaging/linux/shared");
    write(
        &linux.join("app.desktop"),
        "[Desktop Entry]\nType=Application\nName=${APP_DISPLAY_NAME}\n\
         Comment=${APP_DESCRIPTION}\nExec=${APP_BINARY_NAME}\nIcon=${APP_ID}\n\
         Categories=Development;\n",
    );
    write(
        &linux.join("app.metainfo.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<component type=\"desktop-application\">\n  \
         <id>${APP_ID}</id>\n  <name>${APP_DISPLAY_NAME}</name>\n  \
         <launchable type=\"desktop-id\">${APP_ID}.desktop</launchable>\n</component>\n",
    );
    write(
        &linux.join("files/etc/${APP_ID}/app.conf"),
        "version=${APP_VERSION}\n",
    );
    image::DynamicImage::ImageRgba8(image::RgbaImage::new(300, 200))
        .save_with_format(root.join("icon.png"), image::ImageFormat::Png)
        .unwrap();

    // The bundle: an ELF binary (it needs libc, a system library that must
    // stay a dependency) and a shared library (private to the bundle).
    let bundle = root.join("build/bundle");
    std::fs::create_dir_all(bundle.join("lib")).unwrap();
    std::fs::create_dir_all(bundle.join("data")).unwrap();
    let binary = host_file(&["/usr/bin/true", "/bin/true"]).expect("no `true` binary");
    std::fs::copy(binary, bundle.join("hola")).unwrap();
    let library = host_file(&[
        "/usr/lib/libz.so.1",
        "/usr/lib64/libz.so.1",
        "/usr/lib/x86_64-linux-gnu/libz.so.1",
        "/lib/x86_64-linux-gnu/libz.so.1",
        "/usr/lib/aarch64-linux-gnu/libz.so.1",
    ]);
    match library {
        Some(library) => std::fs::copy(library, bundle.join("lib/libapp.so")).map(|_| ()),
        None => std::fs::write(bundle.join("lib/libapp.so"), "not an ELF file"),
    }
    .unwrap();
    write(&bundle.join("data/app.txt"), "data");
    Project { _tmp: tmp, root }
}

fn config(project: &Project, format: &str, version: &str) -> PackageConfig {
    PackageConfig {
        app_name: "hola".into(),
        app_binary_name: "hola".into(),
        app_version: version.into(),
        build_mode: "release".into(),
        platform: Platform::Linux,
        flavor: None,
        channel: None,
        artifact_name: None,
        package_format: format.into(),
        is_installer: false,
        build_output_dir: project.root.join("build/bundle"),
        build_output_files: vec![],
        output_dir: project.root.join("dist"),
        environment: Default::default(),
    }
}

/// Packages `project` in its directory and returns the artifact.
fn package(project: &Project, packager: &dyn AppPackager, format: &str, version: &str) -> PathBuf {
    let _guard = CWD.lock().unwrap_or_else(|e| e.into_inner());
    let previous = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project.root).unwrap();
    let result = packager.package(&config(project, format, version));
    std::env::set_current_dir(previous).unwrap();
    let artifact = result.unwrap().artifacts.remove(0);
    std::path::absolute(project.root.join(artifact)).unwrap()
}

fn has(tool: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {} >/dev/null", tool)])
        .status()
        .is_ok_and(|s| s.success())
}

fn userns() -> bool {
    Command::new("unshare")
        .args(["-r", "true"])
        .status()
        .is_ok_and(|s| s.success())
}

/// Runs `program args` and returns stdout, failing the test on an error.
fn run(program: &str, args: &[&str]) -> String {
    let out = Command::new(program).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{} {:?} failed:\n{}{}",
        program,
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Runs `program args` as root in a user namespace; returns (success, output).
fn as_root(program: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new("unshare")
        .arg("-r")
        .arg(program)
        .args(args)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn as_root_ok(program: &str, args: &[&str]) -> String {
    let (ok, text) = as_root(program, args);
    assert!(ok, "{} {:?} failed:\n{}", program, args, text);
    text
}

fn skip(what: &str) {
    eprintln!("skipped: {}", what);
}

fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
#[ignore = "needs Linux with dpkg-deb (and dpkg + unshare for install checks)"]
fn deb_follows_debian_conventions() {
    if !has("dpkg-deb") {
        return skip("dpkg-deb not found");
    }
    let project = project();
    let v1 = package(&project, &LinuxDebPackager, "deb", "1.2.3-beta.1+4");

    let fields = run("dpkg-deb", &["-f", s(&v1)]);
    assert!(fields.contains("Package: hola\n"), "{}", fields);
    // A pre-release sorts before the release.
    assert!(fields.contains("Version: 1.2.3~beta.1+4\n"), "{}", fields);
    assert!(fields.contains("Maintainer: Test <test@example.com>\n"));
    assert!(fields.contains("Depends: libgtk-3-0\n"));
    assert!(fields.contains("Installed-Size: "));
    assert!(fields.contains("Description: A packaging test app\n"));

    let contents = run("dpkg-deb", &["-c", s(&v1)]);
    for line in contents.lines() {
        assert!(line.contains(" root/root "), "not owned by root: {}", line);
    }
    assert!(contents.contains("./usr/bin/hola -> /opt/hola/hola"));
    assert!(contents.contains("./usr/share/applications/dev.example.hola.desktop"));
    assert!(contents.contains("./usr/share/metainfo/dev.example.hola.metainfo.xml"));
    assert!(contents.contains("./usr/share/icons/hicolor/256x256/apps/dev.example.hola.png"));
    assert!(contents.contains(&format!("./{}", CONF)));

    let control = tempfile::tempdir().unwrap();
    run("dpkg-deb", &["-e", s(&v1), s(control.path())]);
    assert_eq!(
        std::fs::read_to_string(control.path().join("conffiles")).unwrap(),
        format!("/{}\n", CONF)
    );
    let md5sums = std::fs::read_to_string(control.path().join("md5sums")).unwrap();
    assert!(md5sums.contains("  opt/hola/hola\n"));
    assert!(md5sums.contains(&format!("  {}\n", CONF)));
    // The /usr/bin link is part of the package: no maintainer scripts.
    assert!(!control.path().join("postinst").exists());

    if !has("dpkg") || !userns() {
        return skip("deb install checks (dpkg or unshare missing)");
    }
    let target = tempfile::tempdir().unwrap();
    let root = target.path();
    std::fs::create_dir_all(root.join("var/lib/dpkg/info")).unwrap();
    std::fs::create_dir_all(root.join("var/lib/dpkg/updates")).unwrap();
    std::fs::write(root.join("var/lib/dpkg/status"), "").unwrap();
    let dpkg = |args: &[&str]| -> (bool, String) {
        let mut all = vec!["--root", s(root), "--log=/dev/null"];
        all.extend_from_slice(args);
        as_root("dpkg", &all)
    };

    // The GTK dependency is enforced ...
    let (ok, out) = dpkg(&["-i", s(&v1)]);
    assert!(!ok && out.contains("libgtk-3-0"), "{}", out);
    // ... and, forced past it, the package installs.
    let (ok, out) = dpkg(&["--force-depends", "-i", s(&v1)]);
    assert!(ok, "{}", out);
    assert!(root.join("opt/hola/hola").is_file());
    assert_eq!(
        std::fs::read_link(root.join("usr/bin/hola")).unwrap(),
        Path::new("/opt/hola/hola")
    );
    assert_eq!(
        std::fs::read_to_string(root.join(CONF)).unwrap(),
        "version=1.2.3-beta.1+4\n"
    );

    // A local change to a configuration file survives an upgrade.
    std::fs::write(root.join(CONF), "local=1\n").unwrap();
    let v2 = package(&project, &LinuxDebPackager, "deb", "1.2.3+5");
    let (ok, out) = dpkg(&["--force-depends", "--force-confold", "-i", s(&v2)]);
    assert!(ok, "{}", out);
    assert_eq!(
        std::fs::read_to_string(root.join(CONF)).unwrap(),
        "local=1\n"
    );
    let status = std::fs::read_to_string(root.join("var/lib/dpkg/status")).unwrap();
    assert!(status.contains("Version: 1.2.3+5\n"), "{}", status);

    // Removing keeps configuration files; purging deletes them.
    let (ok, out) = dpkg(&["-r", "hola"]);
    assert!(ok, "{}", out);
    assert!(!root.join("opt/hola").exists());
    assert!(root.join("usr/bin/hola").symlink_metadata().is_err());
    assert!(root.join(CONF).is_file());
    let (ok, out) = dpkg(&["-P", "hola"]);
    assert!(ok, "{}", out);
    assert!(!root.join(CONF).exists());
}

#[test]
#[ignore = "needs Linux with rpmbuild (and rpm + unshare for install checks)"]
fn rpm_follows_rpm_conventions() {
    if !has("rpmbuild") {
        return skip("rpmbuild not found");
    }
    let project = project();
    let v1 = package(&project, &LinuxRpmPackager, "rpm", "1.2.3-beta.1+4");

    let info = run(
        "rpm",
        &[
            "-qp",
            "--qf",
            "%{NAME}|%{VERSION}|%{RELEASE}|%{LICENSE}|%{SUMMARY}|%{PACKAGER}\n",
            s(&v1),
        ],
    );
    let release_prefix = "hola|1.2.3~beta.1|4";
    assert!(info.starts_with(release_prefix), "{}", info);
    assert!(info.contains("|MIT|A packaging test app|Test <test@example.com>"));

    // The bundle's own libraries are neither provided nor required, while
    // system libraries (libc here) stay required.
    let provides = run("rpm", &["-qp", "--provides", s(&v1)]);
    assert!(!provides.contains("libz"), "{}", provides);
    let requires = run("rpm", &["-qp", "--requires", s(&v1)]);
    assert!(requires.contains("libc.so.6"), "{}", requires);
    assert!(!requires.contains("libapp"), "{}", requires);

    let files = run("rpm", &["-qlvp", s(&v1)]);
    for line in files.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        assert_eq!((cols[2], cols[3]), ("root", "root"), "{}", line);
        assert!(!cols[0].contains('s'), "setuid/setgid: {}", line);
    }
    assert!(files.contains("/usr/bin/hola -> /opt/hola/hola"));
    assert!(files.contains("/usr/share/applications/dev.example.hola.desktop"));
    // The package owns the directories it creates, not the system's.
    let dirs: Vec<&str> = files
        .lines()
        .filter(|l| l.starts_with('d'))
        .map(|l| l.split_whitespace().last().unwrap())
        .collect();
    assert!(dirs.contains(&"/etc/dev.example.hola"), "{:?}", dirs);
    assert!(dirs.contains(&"/opt/hola"), "{:?}", dirs);
    assert!(!dirs.contains(&"/usr/share/applications"), "{:?}", dirs);
    let config = run("rpm", &["-qcp", s(&v1)]);
    assert_eq!(config.trim(), format!("/{}", CONF));

    if !has("rpm") || !userns() {
        return skip("rpm install checks (rpm or unshare missing)");
    }
    let target = tempfile::tempdir().unwrap();
    let root = target.path();
    // fastforge does not sign packages; rpm 6 refuses unsigned ones unless told.
    let rpm = |args: &[&str]| -> (bool, String) {
        let mut all = vec!["--root", s(root), "--nosignature"];
        all.extend_from_slice(args);
        as_root("rpm", &all)
    };
    as_root_ok("rpm", &["--root", s(root), "--initdb"]);
    let (ok, out) = rpm(&["-i", s(&v1)]);
    assert!(!ok && out.contains("libc.so.6"), "{}", out);
    let (ok, out) = rpm(&["-i", "--nodeps", s(&v1)]);
    assert!(ok, "{}", out);
    assert!(root.join("opt/hola/hola").is_file());
    assert!(root.join("usr/bin/hola").symlink_metadata().is_ok());

    std::fs::write(root.join(CONF), "local=1\n").unwrap();
    let v2 = package(&project, &LinuxRpmPackager, "rpm", "1.2.3+5");
    let (ok, out) = rpm(&["-U", "--nodeps", s(&v2)]);
    assert!(ok, "{}", out);
    // %config(noreplace): the local change stays, the new file is .rpmnew.
    assert_eq!(
        std::fs::read_to_string(root.join(CONF)).unwrap(),
        "local=1\n"
    );
    assert!(root.join(format!("{}.rpmnew", CONF)).is_file());

    let (ok, out) = rpm(&["-e", "hola"]);
    assert!(ok, "{}", out);
    assert!(!root.join("opt/hola").exists());
    assert!(root.join("usr/bin/hola").symlink_metadata().is_err());
    assert!(root.join(format!("{}.rpmsave", CONF)).is_file());
}

#[test]
#[ignore = "needs Linux with bsdtar (and pacman + unshare for install checks)"]
fn pacman_follows_makepkg_conventions() {
    if !has("bsdtar") {
        return skip("bsdtar not found");
    }
    let project = project();
    let v1 = package(&project, &LinuxPacmanPackager, "pacman", "1.2.3-beta.1+4");
    assert!(s(&v1).ends_with(".pkg.tar.zst"), "{}", v1.display());

    let listing = run("bsdtar", &["-tvf", s(&v1)]);
    for line in listing.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        assert_eq!((cols[2], cols[3]), ("root", "root"), "{}", line);
    }
    let pkginfo = run("bsdtar", &["-xOf", s(&v1), ".PKGINFO"]);
    assert!(pkginfo.contains("pkgname = hola\n"), "{}", pkginfo);
    assert!(pkginfo.contains("pkgver = 1.2.3beta.1-4\n"));
    assert!(pkginfo.contains("license = MIT\n"));
    assert!(pkginfo.contains("depend = gtk3\n"));
    assert!(pkginfo.contains(&format!("backup = {}\n", CONF)));
    assert!(pkginfo.contains("\nsize = "));
    assert!(pkginfo.contains("\nbuilddate = "));
    let entries = run("bsdtar", &["-tf", s(&v1)]);
    assert!(entries.starts_with(".MTREE\n.PKGINFO\n"), "{}", entries);
    assert!(!entries.contains(".INSTALL"));

    if !has("pacman") || !userns() {
        return skip("pacman install checks (pacman or unshare missing)");
    }
    let target = tempfile::tempdir().unwrap();
    let root = target.path();
    std::fs::create_dir_all(root.join("var/lib/pacman")).unwrap();
    std::fs::create_dir_all(root.join("hooks")).unwrap();
    let dbpath = root.join("var/lib/pacman");
    let logfile = root.join("pacman.log");
    let hookdir = root.join("hooks");
    let cachedir = root.join("cache");
    std::fs::create_dir_all(&cachedir).unwrap();
    let pacman = |args: &[&str]| -> (bool, String) {
        let mut all = vec![
            "--root",
            s(root),
            "--dbpath",
            s(&dbpath),
            "--logfile",
            s(&logfile),
            "--hookdir",
            s(&hookdir),
            "--cachedir",
            s(&cachedir),
            // The download sandbox switches users, which a user namespace
            // cannot do.
            "--disable-sandbox",
            "--noconfirm",
            "--noscriptlet",
        ];
        all.extend_from_slice(args);
        as_root("pacman", &all)
    };
    let (ok, out) = pacman(&["-U", "--nodeps", "--nodeps", s(&v1)]);
    assert!(ok, "{}", out);
    assert!(root.join("opt/hola/hola").is_file());
    assert!(root.join("usr/bin/hola").symlink_metadata().is_ok());

    std::fs::write(root.join(CONF), "local=1\n").unwrap();
    let v2 = package(&project, &LinuxPacmanPackager, "pacman", "1.2.3+5");
    let (ok, out) = pacman(&["-U", "--nodeps", "--nodeps", s(&v2)]);
    assert!(ok, "{}", out);
    // `backup`: the local change stays, the new file is .pacnew.
    assert_eq!(
        std::fs::read_to_string(root.join(CONF)).unwrap(),
        "local=1\n"
    );
    assert!(root.join(format!("{}.pacnew", CONF)).is_file());

    let (ok, out) = pacman(&["-R", "hola"]);
    assert!(ok, "{}", out);
    assert!(!root.join("opt/hola").exists());
    assert!(root.join("usr/bin/hola").symlink_metadata().is_err());
    assert!(root.join(format!("{}.pacsave", CONF)).is_file());
}

#[test]
#[ignore = "needs Linux with appimagetool"]
fn appimage_lays_out_the_app_dir() {
    if !has("appimagetool") {
        return skip("appimagetool not found");
    }
    let project = project();
    let image = package(&project, &LinuxAppImagePackager, "appimage", "1.2.3+4");
    let extract = tempfile::tempdir().unwrap();
    let out = Command::new(&image)
        .arg("--appimage-extract")
        .current_dir(extract.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let app_dir = extract.path().join("squashfs-root");
    assert!(app_dir.join(format!("{}.desktop", APP_ID)).is_file());
    assert!(app_dir.join(format!("{}.png", APP_ID)).is_file());
    assert!(app_dir.join("AppRun").is_file());
    assert!(
        app_dir
            .join("usr/share/metainfo/dev.example.hola.metainfo.xml")
            .is_file()
    );
    // Only `usr/` of the shared overlay goes into an AppDir.
    assert!(!app_dir.join("etc").exists());
    let desktop = std::fs::read_to_string(app_dir.join(format!("{}.desktop", APP_ID))).unwrap();
    assert!(desktop.contains("Exec=hola\n"), "{}", desktop);
    assert!(desktop.contains("Icon=dev.example.hola\n"));
}
