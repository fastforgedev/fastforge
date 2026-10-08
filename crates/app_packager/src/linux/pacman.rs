use std::path::Path;
use std::process::Command;

use fastforge_core::{
    AppPackager, PackageConfig, PackageError, PackageResult, Platform, Variables,
};
use serde::Deserialize;

use super::common::{
    FormatVariables, Person, RawPackaging, desktop_categories, desktop_list, load_make_config,
    machine_architecture, render_desktop_entry, var,
};
use super::staging::{self, Contents, Layout};

/// The artifact's extension, as `makepkg` names packages.
const PACKAGE_EXTENSION: &str = "pkg.tar.zst";

/// Builds a pacman package (`.pkg.tar.zst`) using `bsdtar`, mirroring
/// Dart's `AppPackageMakerPacman`.
///
/// The package root is staged by [`staging::stage`] (bundle in
/// `/opt/<binary>`, `/usr/bin/<binary>` link, desktop entry, icons,
/// metainfo, `files/` overlays). Raw `PKGINFO` and `INSTALL` files (with or
/// without the leading dot) in `.fastforge/packaging/linux/pacman/` are
/// rendered with fastforge's variables and win over what would be generated
/// from `linux/packaging/pacman/make_config.yaml` (same schema as Dart's
/// `MakePacmanConfig`) or defaults; `size`, `builddate` and the `backup`
/// entries of `/etc` files are filled in. Files are archived as owned by root, like `makepkg` under
/// `fakeroot`.
///
/// Requires `bsdtar` (libarchive, with zstd support) on `$PATH`.
pub struct LinuxPacmanPackager;

/// Schema of `linux/packaging/pacman/make_config.yaml`, mirroring Dart's
/// `MakePacmanConfig.fromJson`. `installed_size` and `options` (a `makepkg`
/// setting) are accepted but unused: the size is computed.
#[derive(Debug, Default, Deserialize)]
pub struct PacmanMakeConfig {
    pub display_name: Option<String>,
    pub package_name: Option<String>,
    pub maintainer: Option<Person>,
    pub installed_size: Option<i64>,
    pub licenses: Option<Vec<String>>,
    pub groups: Option<Vec<String>>,
    pub options: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    pub optional_dependencies: Option<Vec<String>>,
    pub conflicts: Option<Vec<String>>,
    pub replaces: Option<Vec<String>>,
    pub provides: Option<Vec<String>>,
    pub postinstall_scripts: Option<Vec<String>>,
    pub postupgrade_scripts: Option<Vec<String>>,
    pub postuninstall_scripts: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub supported_mime_type: Option<Vec<String>>,
    pub actions: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub generic_name: Option<String>,
    pub startup_notify: Option<bool>,
    pub icon: Option<String>,
    pub metainfo: Option<String>,
}

/// pacman architecture from `uname -m`.
fn pacman_architecture() -> String {
    machine_architecture().to_string()
}

/// `pkgver-pkgrel` from the app version (`1.2.3+4` -> `1.2.3-4`). pkgver may
/// not contain `-`, so a pre-release `1.2.3-beta.1` becomes `1.2.3beta.1`,
/// which pacman sorts before `1.2.3`.
fn pacman_version(app_version: &str) -> String {
    let (name, build) = app_version.split_once('+').unwrap_or((app_version, ""));
    let release = build
        .split('.')
        .next()
        .filter(|r| !r.is_empty())
        .unwrap_or("1");
    format!("{}-{}", name.replace('-', ""), release)
}

impl PacmanMakeConfig {
    fn load() -> Result<Self, PackageError> {
        Ok(
            load_make_config(Path::new("linux/packaging/pacman/make_config.yaml"))?
                .unwrap_or_default(),
        )
    }

    /// Renders the default `.PKGINFO` in `makepkg`'s format (`key = value`,
    /// one line per list item). Project metadata fills what
    /// `make_config.yaml` leaves out; `size` and `builddate` are added by
    /// [`fill_pkginfo`].
    fn pkginfo_file(&self, config: &PackageConfig, variables: &Variables) -> String {
        let name = var(variables, "PACKAGE_NAME").unwrap_or_else(|| config.app_binary_name.clone());
        let description = var(variables, "APP_DESCRIPTION")
            .or_else(|| var(variables, "APP_DISPLAY_NAME"))
            .unwrap_or_else(|| config.app_name.clone());
        let packager = self
            .maintainer
            .as_ref()
            .map(Person::formatted)
            .or_else(|| var(variables, "APP_MAINTAINER"))
            .unwrap_or_else(|| "Unknown Packager".to_string());
        let licenses = self
            .licenses
            .clone()
            .filter(|v| !v.is_empty())
            .or_else(|| var(variables, "APP_LICENSE").map(|l| vec![l]))
            .unwrap_or_else(|| vec!["LicenseRef-Unknown".to_string()]);

        let mut lines = vec![
            format!("pkgname = {}", name),
            format!("pkgbase = {}", name),
            "xdata = pkgtype=pkg".to_string(),
            format!(
                "pkgver = {}",
                var(variables, "PACKAGE_VERSION")
                    .unwrap_or_else(|| pacman_version(&config.app_version))
            ),
            format!("pkgdesc = {}", description),
        ];
        if let Some(url) = var(variables, "APP_HOMEPAGE") {
            lines.push(format!("url = {}", url));
        }
        lines.push(format!("packager = {}", packager));
        lines.push(format!("arch = {}", pacman_architecture()));
        let mut list = |key: &str, values: &[String]| {
            for value in values {
                lines.push(format!("{} = {}", key, value));
            }
        };
        list("license", &licenses);
        // A Flutter app needs GTK; an empty list opts out.
        let depends = self
            .dependencies
            .clone()
            .unwrap_or_else(|| vec!["gtk3".to_string()]);
        for (key, values) in [
            ("group", &self.groups),
            ("conflict", &self.conflicts),
            ("provides", &self.provides),
            ("replaces", &self.replaces),
            ("depend", &Some(depends)),
            ("optdepend", &self.optional_dependencies),
        ] {
            list(key, values.as_deref().unwrap_or_default());
        }
        lines.join("\n") + "\n"
    }

    /// Renders `.INSTALL` from the configured scripts, `None` when there are
    /// none (the `/usr/bin` link is part of the package, not created by a
    /// script).
    fn install_file(&self) -> Option<String> {
        let mut sections = Vec::new();
        for (function, scripts) in [
            ("post_install", &self.postinstall_scripts),
            ("post_upgrade", &self.postupgrade_scripts),
            ("post_remove", &self.postuninstall_scripts),
        ] {
            if let Some(scripts) = scripts.as_ref().filter(|s| !s.is_empty()) {
                sections.push(format!(
                    "{}() {{\n\t{}\n}}\n",
                    function,
                    scripts.join("\n\t")
                ));
            }
        }
        (!sections.is_empty()).then(|| sections.join("\n"))
    }

    /// Renders the default desktop entry.
    fn desktop_file(&self, config: &PackageConfig, icon: &str) -> String {
        let binary_name = &config.app_binary_name;
        render_desktop_entry(&[
            ("Type", Some("Application".to_string())),
            (
                "Name",
                Some(
                    self.display_name
                        .clone()
                        .unwrap_or_else(|| config.app_name.clone()),
                ),
            ),
            ("GenericName", self.generic_name.clone()),
            ("Icon", Some(icon.to_string())),
            ("Exec", Some(format!("{} %U", binary_name))),
            ("Actions", desktop_list(&self.actions)),
            ("MimeType", desktop_list(&self.supported_mime_type)),
            ("Categories", desktop_categories(&self.categories)),
            ("Keywords", desktop_list(&self.keywords)),
            (
                "StartupNotify",
                Some(self.startup_notify.unwrap_or(false).to_string()),
            ),
        ])
    }
}

/// Adds `size` (installed bytes) and `builddate` to a `.PKGINFO` that does
/// not set them, a `backup` entry for every configuration file (`/etc`) so
/// pacman keeps local changes, and makes sure it ends with a newline.
fn fill_pkginfo(pkginfo: &str, size: u64, build_date: u64, config_files: &[String]) -> String {
    let mut out = pkginfo.trim_end_matches('\n').to_string();
    out.push('\n');
    let has = |key: &str| {
        pkginfo
            .lines()
            .any(|line| line.split_once('=').is_some_and(|(k, _)| k.trim() == key))
    };
    if !has("builddate") {
        out.push_str(&format!("builddate = {}\n", build_date));
    }
    if !has("size") {
        out.push_str(&format!("size = {}\n", size));
    }
    for file in config_files {
        let entry = file.trim_start_matches('/');
        let listed = pkginfo.lines().any(|line| {
            line.split_once('=')
                .is_some_and(|(k, v)| k.trim() == "backup" && v.trim() == entry)
        });
        if !listed {
            out.push_str(&format!("backup = {}\n", entry));
        }
    }
    out
}

fn run(cmd: &mut Command) -> Result<(), PackageError> {
    let out = cmd.output().map_err(|e| {
        PackageError::MissingTool(format!("{}: {}", cmd.get_program().to_string_lossy(), e))
    })?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: cmd.get_program().to_string_lossy().into(),
            stderr: String::from_utf8_lossy(&out.stderr).into(),
        });
    }
    Ok(())
}

/// `bsdtar` options recording every file as owned by root.
const ROOT_OWNER: [&str; 8] = [
    "--uid", "0", "--gid", "0", "--uname", "root", "--gname", "root",
];

impl AppPackager for LinuxPacmanPackager {
    fn name(&self) -> &str {
        "pacman"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        PACKAGE_EXTENSION
    }

    #[cfg(not(target_os = "linux"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = PacmanMakeConfig::load()?;
        // The artifact uses makepkg's `.pkg.tar.zst` extension.
        let mut effective = config.clone();
        effective.package_format = PACKAGE_EXTENSION.to_string();
        let pkg_dir = effective.packaging_dir();
        let output_file = effective.output_file();
        // bsdtar runs in the package root, so it needs an absolute path.
        let archive_path = std::path::absolute(&output_file)?;
        let binary_name = &config.app_binary_name;
        let layout = Layout::system(binary_name);
        let raw = RawPackaging::load(
            &effective,
            "pacman",
            FormatVariables {
                package_name: make_config.package_name.clone(),
                default_package_name: binary_name.clone(),
                package_version: pacman_version(&config.app_version),
                package_arch: pacman_architecture(),
                install_dir: Some(layout.install_dir()),
                display_name: make_config.display_name.clone(),
                packaging_dir: &pkg_dir,
                output_file: &output_file,
                extra: vec![],
            },
        )?;

        staging::stage(
            &raw,
            config,
            &pkg_dir,
            &layout,
            Contents {
                icon: raw.icon(make_config.icon.as_ref()),
                metainfo: make_config.metainfo.clone(),
            },
            || make_config.desktop_file(config, raw.app_id()),
        )?;
        let (size, _) = staging::installed_size(&pkg_dir, &[])?;

        // .PKGINFO (with size/builddate) and .INSTALL
        let pkginfo = match raw.file(&["PKGINFO", ".PKGINFO"]) {
            Some(src) => raw.render(&src)?,
            None => make_config.pkginfo_file(config, raw.variables()),
        };
        std::fs::write(
            pkg_dir.join(".PKGINFO"),
            fill_pkginfo(
                &pkginfo,
                size,
                staging::build_date(),
                &staging::config_files(&pkg_dir)?,
            ),
        )?;
        let install = match raw.file(&["INSTALL", ".INSTALL"]) {
            Some(src) => Some(raw.render(&src)?),
            None => make_config.install_file(),
        };
        if let Some(install) = &install {
            std::fs::write(pkg_dir.join(".INSTALL"), install)?;
        }

        // The metadata files, then every top-level entry of the root.
        let mut contents: Vec<String> = std::fs::read_dir(&pkg_dir)?
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name != ".PKGINFO" && name != ".INSTALL")
            .collect();
        contents.sort();
        let mut entries = vec![".PKGINFO".to_string()];
        if install.is_some() {
            entries.push(".INSTALL".to_string());
        }
        entries.extend(contents);

        // .MTREE metadata, then the zstd-compressed archive.
        let mut mtree = Command::new("bsdtar");
        mtree
            .current_dir(&pkg_dir)
            .args([
                "-czf",
                ".MTREE",
                "--format=mtree",
                "--options=!all,use-set,type,uid,gid,mode,time,size,md5,sha256,link",
            ])
            .args(ROOT_OWNER)
            .args(&entries)
            .env("LANG", "C");
        run(&mut mtree)?;

        let mut archive = Command::new("bsdtar");
        archive
            .current_dir(&pkg_dir)
            .args(["-c", "--zstd", "-f"])
            .arg(&archive_path)
            .args(ROOT_OWNER)
            .arg(".MTREE")
            .args(&entries)
            .env("LANG", "C");
        raw.apply_env(&mut archive);
        run(&mut archive)?;

        std::fs::remove_dir_all(&pkg_dir).ok();
        effective.resolve_result(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config() -> PackageConfig {
        PackageConfig {
            app_name: "hola_amigos".into(),
            app_binary_name: "hola_amigos".into(),
            app_version: "1.2.3+4".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "pacman".into(),
            is_installer: false,
            build_output_dir: PathBuf::new(),
            build_output_files: vec![],
            output_dir: PathBuf::new(),
            environment: Default::default(),
        }
    }

    fn full_make_config() -> PacmanMakeConfig {
        serde_yaml::from_str(
            r#"
display_name: Hola Amigos
package_name: hola-amigos
maintainer:
  name: Gamer Boy 69
  email: rickastley@gmail.lol
installed_size: 24400
licenses:
  - MIT
dependencies:
  - mysupercooldep
optional_dependencies:
  - iamalwaysoptional
options:
  - zipman
conflicts:
  - libwhatsup
replaces:
  - yourdep
provides:
  - libx11
postinstall_scripts:
  - echo Installed
postupgrade_scripts:
  - echo Upgraded
postuninstall_scripts:
  - echo Removed
categories:
  - Music
startup_notify: true
"#,
        )
        .unwrap()
    }

    #[test]
    fn pkginfo_uses_makepkg_format() {
        let variables: Variables = [("PACKAGE_NAME", "hola-amigos")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let pkginfo = full_make_config().pkginfo_file(&test_config(), &variables);
        assert_eq!(
            pkginfo,
            format!(
                "pkgname = hola-amigos\n\
                 pkgbase = hola-amigos\n\
                 xdata = pkgtype=pkg\n\
                 pkgver = 1.2.3-4\n\
                 pkgdesc = hola_amigos\n\
                 packager = Gamer Boy 69 <rickastley@gmail.lol>\n\
                 arch = {}\n\
                 license = MIT\n\
                 conflict = libwhatsup\n\
                 provides = libx11\n\
                 replaces = yourdep\n\
                 depend = mysupercooldep\n\
                 optdepend = iamalwaysoptional\n",
                pacman_architecture()
            )
        );
    }

    #[test]
    fn pkginfo_defaults_come_from_the_project() {
        let variables: Variables = [
            ("APP_DESCRIPTION", "A demo"),
            ("APP_HOMEPAGE", "https://example.com"),
            ("APP_MAINTAINER", "Jane <jane@example.com>"),
            ("APP_LICENSE", "Apache-2.0"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let pkginfo = PacmanMakeConfig::default().pkginfo_file(&test_config(), &variables);
        assert!(pkginfo.contains("pkgname = hola_amigos\n"));
        assert!(pkginfo.contains("pkgdesc = A demo\n"));
        assert!(pkginfo.contains("url = https://example.com\n"));
        assert!(pkginfo.contains("packager = Jane <jane@example.com>\n"));
        assert!(pkginfo.contains("license = Apache-2.0\n"));
        // A Flutter app needs GTK unless dependencies are configured.
        assert!(pkginfo.contains("depend = gtk3\n"));
        let none: PacmanMakeConfig = serde_yaml::from_str("dependencies: []\n").unwrap();
        assert!(
            !none
                .pkginfo_file(&test_config(), &variables)
                .contains("depend")
        );

        let bare = PacmanMakeConfig::default().pkginfo_file(&test_config(), &Variables::new());
        assert!(bare.contains("packager = Unknown Packager\n"));
        assert!(bare.contains("license = LicenseRef-Unknown\n"));
        assert!(!bare.contains("url ="));
    }

    #[test]
    fn versions_follow_pacman_rules() {
        assert_eq!(pacman_version("1.2.3+4"), "1.2.3-4");
        assert_eq!(pacman_version("1.2.3"), "1.2.3-1");
        assert_eq!(pacman_version("1.2.3-beta.1+7"), "1.2.3beta.1-7");
    }

    #[test]
    fn size_and_builddate_are_filled_in() {
        let raw = "pkgname = x\npkgver = 1-1";
        assert_eq!(
            fill_pkginfo(raw, 2048, 1700000000, &[]),
            "pkgname = x\npkgver = 1-1\nbuilddate = 1700000000\nsize = 2048\n"
        );
        let set = "pkgname = x\nsize = 1\nbuilddate=5\n";
        assert_eq!(fill_pkginfo(set, 2048, 1700000000, &[]), set);

        // Configuration files are backed up, once.
        let etc = vec!["/etc/x/a.conf".to_string(), "/etc/x/b.conf".to_string()];
        let listed = "pkgname = x\nsize = 1\nbuilddate = 5\nbackup = etc/x/b.conf\n";
        assert_eq!(
            fill_pkginfo(listed, 2048, 1700000000, &etc),
            format!("{}backup = etc/x/a.conf\n", listed)
        );
    }

    #[test]
    fn install_file_sections() {
        let install = full_make_config().install_file().unwrap();
        assert_eq!(
            install,
            "post_install() {\n\techo Installed\n}\n\n\
             post_upgrade() {\n\techo Upgraded\n}\n\n\
             post_remove() {\n\techo Removed\n}\n"
        );
        assert!(!install.contains("/usr/bin"));
        assert_eq!(PacmanMakeConfig::default().install_file(), None);
    }

    #[test]
    fn desktop_defaults() {
        let desktop = PacmanMakeConfig::default().desktop_file(&test_config(), "hola_amigos");
        assert!(desktop.contains("Name=hola_amigos"));
        assert!(desktop.contains("StartupNotify=false"));
    }
}
