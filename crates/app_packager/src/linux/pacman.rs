use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

use super::common::{
    Person, desktop_list, install_hicolor_icons, install_metainfo, load_make_config,
    load_pubspec_meta, render_desktop_entry, uname_machine,
};

/// Builds a pacman `.pkg.tar.xz` using `bsdtar` and `xz`, mirroring
/// Dart's `AppPackageMakerPacman`.
///
/// Reads `linux/packaging/pacman/make_config.yaml` when present (same schema
/// as Dart's `MakePacmanConfig`); falls back to sensible defaults otherwise.
///
/// Requires `bsdtar` (libarchive) and `xz` to be on `$PATH`.
pub struct LinuxPacmanPackager;

/// Schema of `linux/packaging/pacman/make_config.yaml`, mirroring Dart's
/// `MakePacmanConfig.fromJson`.
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
    if uname_machine() == "aarch64" {
        "aarch64".to_string()
    } else {
        "x86_64".to_string()
    }
}

impl PacmanMakeConfig {
    fn load() -> Result<Self, PackageError> {
        Ok(
            load_make_config(Path::new("linux/packaging/pacman/make_config.yaml"))?
                .unwrap_or_default(),
        )
    }

    /// Renders `.PKGINFO`, mirroring Dart's `toFilesString()['PKGINFO']`.
    fn pkginfo_file(&self, config: &PackageConfig) -> String {
        let meta = load_pubspec_meta();
        let paren = |v: &Option<Vec<String>>| -> Option<String> {
            v.as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| format!("({})", v.join(", ")))
        };
        let licenses = self
            .licenses
            .clone()
            .unwrap_or_else(|| vec!["unknown".to_string()]);
        let groups = self
            .groups
            .clone()
            .unwrap_or_else(|| vec!["default".to_string()]);

        let entries: Vec<(&str, Option<String>)> = vec![
            (
                "pkgname",
                Some(
                    self.package_name
                        .clone()
                        .unwrap_or_else(|| config.app_binary_name.clone()),
                ),
            ),
            ("pkgver", Some(config.app_version.clone())),
            (
                "pkgdesc",
                Some(meta.description.unwrap_or_else(|| config.app_name.clone())),
            ),
            ("packager", self.maintainer.as_ref().map(Person::formatted)),
            ("size", self.installed_size.map(|s| s.to_string())),
            ("license", Some(format!("({})", licenses.join(", ")))),
            ("groups", Some(format!("({})", groups.join(", ")))),
            ("arch", Some(format!("({})", pacman_architecture()))),
            ("url", meta.homepage),
            ("options", paren(&self.options)),
            ("depends", paren(&self.dependencies)),
            ("optdepends", paren(&self.optional_dependencies)),
            ("conflicts", paren(&self.conflicts)),
            ("replaces", paren(&self.replaces)),
            ("provides", paren(&self.provides)),
        ];

        let mut out = String::new();
        for (key, value) in entries {
            if let Some(value) = value {
                out.push_str(&format!("{}={}\n", key, value));
            }
        }
        out
    }

    /// Renders `.INSTALL`, mirroring Dart's `toFilesString()['INSTALL']`.
    fn install_file(&self, binary_name: &str) -> String {
        let mut post_install = vec![
            format!("ln -s /opt/{n}/{n} /usr/bin/{n}", n = binary_name),
            format!("chmod +x /usr/bin/{}", binary_name),
        ];
        post_install.extend(self.postinstall_scripts.clone().unwrap_or_default());

        let mut post_remove = vec![format!("rm /usr/bin/{}", binary_name)];
        post_remove.extend(self.postuninstall_scripts.clone().unwrap_or_default());

        let mut sections = vec![format!(
            "post_install() {{\n\t{}\n}}",
            post_install.join("\n\t")
        )];
        if let Some(upgrade) = self
            .postupgrade_scripts
            .as_ref()
            .filter(|v| !v.is_empty())
        {
            sections.push(format!("post_upgrade() {{\n\t{}\n}}", upgrade.join("\n")));
        }
        sections.push(format!(
            "post_remove() {{\n\t{}\n}}",
            post_remove.join("\n")
        ));
        sections.join("\n")
    }

    /// Renders the desktop entry, mirroring Dart's `toJson()['DESKTOP']`.
    fn desktop_file(&self, config: &PackageConfig) -> String {
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
            ("Icon", Some(binary_name.clone())),
            ("Exec", Some(format!("{} %U", binary_name))),
            ("Actions", desktop_list(&self.actions)),
            ("MimeType", desktop_list(&self.supported_mime_type)),
            ("Categories", desktop_list(&self.categories)),
            ("Keywords", desktop_list(&self.keywords)),
            (
                "StartupNotify",
                Some(self.startup_notify.unwrap_or(false).to_string()),
            ),
        ])
    }
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

impl AppPackager for LinuxPacmanPackager {
    fn name(&self) -> &str {
        "pacman"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        "pacman"
    }

    #[cfg(not(target_os = "linux"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = PacmanMakeConfig::load()?;
        let pkg_dir = config.packaging_dir();
        let binary_name = &config.app_binary_name;

        // Create directory tree
        let share_app_dir = pkg_dir.join("opt").join(binary_name);
        let applications_dir = pkg_dir.join("usr/share/applications");
        std::fs::create_dir_all(&share_app_dir)?;
        std::fs::create_dir_all(&applications_dir)?;

        // Install the configured icon and metainfo (when provided)
        if let Some(icon) = &make_config.icon {
            install_hicolor_icons(icon, &pkg_dir, binary_name)?;
        }
        if let Some(metainfo) = &make_config.metainfo {
            install_metainfo(metainfo, &pkg_dir, binary_name)?;
        }

        // Copy the flutter build output into /opt/{binary_name}/
        run(Command::new("cp").args([
            "-fr",
            &format!("{}/.", config.build_output_dir.display()),
            &share_app_dir.display().to_string(),
        ]))?;

        // Write .PKGINFO, .INSTALL, .desktop
        std::fs::write(
            pkg_dir.join(".PKGINFO"),
            make_config.pkginfo_file(config),
        )?;
        std::fs::write(
            pkg_dir.join(".INSTALL"),
            make_config.install_file(binary_name),
        )?;
        std::fs::write(
            applications_dir.join(format!("{}.desktop", binary_name)),
            make_config.desktop_file(config),
        )?;

        // Create .MTREE metadata
        run(Command::new("bsdtar")
            .current_dir(&pkg_dir)
            .args([
                "-czf",
                ".MTREE",
                "--format=mtree",
                "--options=!all,use-set,type,uid,gid,mode,time,size,md5,sha256,link",
                ".PKGINFO",
                ".INSTALL",
                "usr",
                "opt",
            ])
            .env("LANG", "C"))?;

        // Archive with bsdtar
        run(Command::new("bsdtar")
            .current_dir(&pkg_dir)
            .args([
                "-cf", "temptar", ".MTREE", ".INSTALL", ".PKGINFO", "usr", "opt",
            ])
            .env("LANG", "C"))?;

        // Compress with xz
        run(Command::new("xz")
            .current_dir(&pkg_dir)
            .args(["-z", "temptar"]))?;

        // Move to output
        let output_file = config.output_file();
        std::fs::rename(pkg_dir.join("temptar.xz"), &output_file)?;

        std::fs::remove_dir_all(&pkg_dir).ok();
        Ok(PackageResult {
            artifacts: vec![output_file],
        })
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
    fn pkginfo_contains_configured_fields() {
        let pkginfo = full_make_config().pkginfo_file(&test_config());
        assert!(pkginfo.contains("pkgname=hola-amigos"));
        assert!(pkginfo.contains("pkgver=1.2.3+4"));
        assert!(pkginfo.contains("packager=Gamer Boy 69 <rickastley@gmail.lol>"));
        assert!(pkginfo.contains("size=24400"));
        assert!(pkginfo.contains("license=(MIT)"));
        assert!(pkginfo.contains("groups=(default)"));
        assert!(pkginfo.contains("options=(zipman)"));
        assert!(pkginfo.contains("depends=(mysupercooldep)"));
        assert!(pkginfo.contains("optdepends=(iamalwaysoptional)"));
        assert!(pkginfo.contains("conflicts=(libwhatsup)"));
        assert!(pkginfo.contains("replaces=(yourdep)"));
        assert!(pkginfo.contains("provides=(libx11)"));
    }

    #[test]
    fn install_file_sections() {
        let install = full_make_config().install_file("hola_amigos");
        assert!(install.contains("post_install() {"));
        assert!(install.contains("ln -s /opt/hola_amigos/hola_amigos /usr/bin/hola_amigos"));
        assert!(install.contains("echo Installed"));
        assert!(install.contains("post_upgrade() {"));
        assert!(install.contains("echo Upgraded"));
        assert!(install.contains("post_remove() {"));
        assert!(install.contains("rm /usr/bin/hola_amigos"));
        assert!(install.contains("echo Removed"));
    }

    #[test]
    fn install_file_omits_empty_upgrade_section() {
        let install = PacmanMakeConfig::default().install_file("demo");
        assert!(!install.contains("post_upgrade"));
    }

    #[test]
    fn desktop_defaults() {
        let desktop = PacmanMakeConfig::default().desktop_file(&test_config());
        assert!(desktop.contains("Name=hola_amigos"));
        assert!(desktop.contains("StartupNotify=false"));
    }
}
