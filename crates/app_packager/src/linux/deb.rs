use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

use super::common::{
    Person, deb_architecture, desktop_list, install_hicolor_icons, install_metainfo,
    load_make_config, load_pubspec_meta, render_desktop_entry,
};

/// Builds a Debian `.deb` package using `dpkg-deb`, mirroring
/// Dart's `AppPackageMakerDeb`.
///
/// Reads `linux/packaging/deb/make_config.yaml` when present (same schema as
/// Dart's `MakeDebConfig`); falls back to sensible defaults otherwise.
///
/// Requires `dpkg-deb` to be installed on the host (`dpkg-dev` on Debian/Ubuntu).
pub struct LinuxDebPackager;

/// Schema of `linux/packaging/deb/make_config.yaml`, mirroring Dart's
/// `MakeDebConfig.fromJson`.
#[derive(Debug, Default, Deserialize)]
pub struct DebMakeConfig {
    pub display_name: Option<String>,
    pub package_name: Option<String>,
    pub maintainer: Option<Person>,
    pub co_authors: Option<Vec<Person>>,
    pub priority: Option<String>,
    pub section: Option<String>,
    pub installed_size: Option<i64>,
    pub essential: Option<bool>,
    pub dependencies: Option<Vec<String>>,
    pub build_dependencies_indep: Option<Vec<String>>,
    pub build_dependencies: Option<Vec<String>>,
    pub recommended_dependencies: Option<Vec<String>>,
    pub suggested_dependencies: Option<Vec<String>>,
    pub enhances: Option<Vec<String>>,
    pub pre_dependencies: Option<Vec<String>>,
    pub breaks: Option<Vec<String>>,
    pub conflicts: Option<Vec<String>>,
    pub provides: Option<Vec<String>>,
    pub replaces: Option<Vec<String>>,
    pub postinstall_scripts: Option<Vec<String>>,
    pub postuninstall_scripts: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub supported_mime_type: Option<Vec<String>>,
    pub actions: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub generic_name: Option<String>,
    pub startup_notify: Option<bool>,
    pub startup_wm_class: Option<String>,
    pub icon: Option<String>,
    pub metainfo: Option<String>,
}

impl DebMakeConfig {
    fn load() -> Result<Self, PackageError> {
        Ok(
            load_make_config(Path::new("linux/packaging/deb/make_config.yaml"))?
                .unwrap_or_default(),
        )
    }

    /// Renders the `DEBIAN/control` file, mirroring Dart's `toJson()['CONTROL']`.
    fn control_file(&self, config: &PackageConfig) -> String {
        let meta = load_pubspec_meta();
        let join = |v: &Option<Vec<String>>| -> Option<String> {
            v.as_ref().filter(|v| !v.is_empty()).map(|v| v.join(", "))
        };

        let entries: Vec<(&str, Option<String>)> = vec![
            ("Maintainer", self.maintainer.as_ref().map(Person::formatted)),
            (
                "Package",
                Some(
                    self.package_name
                        .clone()
                        .unwrap_or_else(|| config.app_binary_name.clone()),
                ),
            ),
            ("Version", Some(config.app_version.clone())),
            (
                "Section",
                Some(self.section.clone().unwrap_or_else(|| "x11".to_string())),
            ),
            (
                "Priority",
                Some(
                    self.priority
                        .clone()
                        .unwrap_or_else(|| "optional".to_string()),
                ),
            ),
            ("Architecture", Some(deb_architecture().to_string())),
            (
                "Essential",
                self.essential
                    .map(|e| (if e { "yes" } else { "no" }).to_string()),
            ),
            ("Installed-Size", self.installed_size.map(|s| s.to_string())),
            (
                "Description",
                Some(meta.description.unwrap_or_else(|| config.app_name.clone())),
            ),
            ("Homepage", meta.homepage),
            ("Depends", join(&self.dependencies)),
            ("Build-Depends-Indep", join(&self.build_dependencies_indep)),
            ("Build-Depends", join(&self.build_dependencies)),
            ("Pre-Depends", join(&self.pre_dependencies)),
            ("Recommends", join(&self.recommended_dependencies)),
            ("Suggests", join(&self.suggested_dependencies)),
            ("Enhances", join(&self.enhances)),
            ("Breaks", join(&self.breaks)),
            ("Conflicts", join(&self.conflicts)),
            ("Provides", join(&self.provides)),
            ("Replaces", join(&self.replaces)),
            (
                "Uploaders",
                self.co_authors.as_ref().filter(|v| !v.is_empty()).map(|v| {
                    v.iter()
                        .map(Person::formatted)
                        .collect::<Vec<_>>()
                        .join(", ")
                }),
            ),
        ];

        let mut out = String::new();
        for (key, value) in entries {
            if let Some(value) = value {
                out.push_str(&format!("{}: {}\n", key, value));
            }
        }
        out
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
                Some(self.startup_notify.unwrap_or(true).to_string()),
            ),
            ("StartupWMClass", self.startup_wm_class.clone()),
        ])
    }

    /// Post-install script, always prefixed with the `/usr/bin` symlink setup
    /// (mirrors Dart's `postinstallScripts` getter).
    fn postinst(&self, binary_name: &str) -> String {
        let mut lines = vec![
            "#!/usr/bin/env sh".to_string(),
            format!("ln -s /opt/{n}/{n} /usr/bin/{n}", n = binary_name),
            format!("chmod +x /usr/bin/{}", binary_name),
        ];
        lines.extend(self.postinstall_scripts.clone().unwrap_or_default());
        lines.push("exit 0".to_string());
        lines.join("\n")
    }

    /// Post-uninstall script (mirrors Dart's `postuninstallScripts` getter).
    fn postrm(&self, binary_name: &str) -> String {
        let mut lines = vec![
            "#!/usr/bin/env sh".to_string(),
            format!("rm /usr/bin/{}", binary_name),
        ];
        lines.extend(self.postuninstall_scripts.clone().unwrap_or_default());
        lines.push("exit 0".to_string());
        lines.join("\n")
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

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), PackageError> {
    run(Command::new("cp").args([
        "-fr",
        &format!("{}/.", src.display()),
        &dst.display().to_string(),
    ]))?;
    Ok(())
}

impl AppPackager for LinuxDebPackager {
    fn name(&self) -> &str {
        "deb"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        "deb"
    }

    #[cfg(not(target_os = "linux"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = DebMakeConfig::load()?;
        let pkg_dir = config.packaging_dir();
        let binary_name = &config.app_binary_name;

        // Create the required directory tree
        let debian_dir = pkg_dir.join("DEBIAN");
        let share_app_dir = pkg_dir.join("opt").join(binary_name);
        let applications_dir = pkg_dir.join("usr/share/applications");
        std::fs::create_dir_all(&debian_dir)?;
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
        copy_dir_contents(&config.build_output_dir, &share_app_dir)?;

        // DEBIAN/control
        std::fs::write(
            debian_dir.join("control"),
            make_config.control_file(config),
        )?;

        // DEBIAN/postinst + DEBIAN/postrm
        let postinst_path = debian_dir.join("postinst");
        std::fs::write(&postinst_path, make_config.postinst(binary_name))?;
        run(Command::new("chmod").args(["+x", &postinst_path.display().to_string()]))?;

        let postrm_path = debian_dir.join("postrm");
        std::fs::write(&postrm_path, make_config.postrm(binary_name))?;
        run(Command::new("chmod").args(["+x", &postrm_path.display().to_string()]))?;

        // usr/share/applications/{binary_name}.desktop
        std::fs::write(
            applications_dir.join(format!("{}.desktop", binary_name)),
            make_config.desktop_file(config),
        )?;

        let output_file = config.output_file();
        run(Command::new("dpkg-deb").args([
            "--build",
            "--root-owner-group",
            &pkg_dir.display().to_string(),
            &output_file.display().to_string(),
        ]))?;

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
            package_format: "deb".into(),
            is_installer: false,
            build_output_dir: PathBuf::new(),
            build_output_files: vec![],
            output_dir: PathBuf::new(),
        }
    }

    fn full_make_config() -> DebMakeConfig {
        serde_yaml::from_str(
            r#"
display_name: Hola Amigos
package_name: hola-amigos
maintainer:
  name: Gamer Boy 69
  email: rickastley@gmail.lol
co_authors:
  - name: Mir Jafar
    email: contributor@gmail.com
priority: optional
section: x11
installed_size: 24400
dependencies:
  - libkeybinder-3.0-0 (>= 0.3.2)
essential: false
postinstall_scripts:
  - echo Installed
postuninstall_scripts:
  - echo Removed
keywords:
  - Hello
  - World
generic_name: Hobby Application
supported_mime_type:
  - audio/mpeg
actions:
  - Gallery
categories:
  - Music
  - Media
startup_notify: true
"#,
        )
        .unwrap()
    }

    #[test]
    fn control_file_contains_all_fields() {
        let control = full_make_config().control_file(&test_config());
        assert!(control.contains("Maintainer: Gamer Boy 69 <rickastley@gmail.lol>"));
        assert!(control.contains("Package: hola-amigos"));
        assert!(control.contains("Version: 1.2.3+4"));
        assert!(control.contains("Section: x11"));
        assert!(control.contains("Priority: optional"));
        assert!(control.contains("Essential: no"));
        assert!(control.contains("Installed-Size: 24400"));
        assert!(control.contains("Depends: libkeybinder-3.0-0 (>= 0.3.2)"));
        assert!(control.contains("Uploaders: Mir Jafar <contributor@gmail.com>"));
    }

    #[test]
    fn desktop_file_contains_lists() {
        let desktop = full_make_config().desktop_file(&test_config());
        assert!(desktop.contains("Name=Hola Amigos"));
        assert!(desktop.contains("GenericName=Hobby Application"));
        assert!(desktop.contains("Exec=hola_amigos %U"));
        assert!(desktop.contains("Actions=Gallery;"));
        assert!(desktop.contains("MimeType=audio/mpeg;"));
        assert!(desktop.contains("Categories=Music;Media;"));
        assert!(desktop.contains("Keywords=Hello;World;"));
        assert!(desktop.contains("StartupNotify=true"));
    }

    #[test]
    fn scripts_include_symlink_setup_and_custom_lines() {
        let mc = full_make_config();
        let postinst = mc.postinst("hola_amigos");
        assert!(postinst.contains("ln -s /opt/hola_amigos/hola_amigos /usr/bin/hola_amigos"));
        assert!(postinst.contains("echo Installed"));
        assert!(postinst.ends_with("exit 0"));
        let postrm = mc.postrm("hola_amigos");
        assert!(postrm.contains("rm /usr/bin/hola_amigos"));
        assert!(postrm.contains("echo Removed"));
    }

    #[test]
    fn defaults_without_make_config() {
        let mc = DebMakeConfig::default();
        let control = mc.control_file(&test_config());
        assert!(control.contains("Package: hola_amigos"));
        assert!(control.contains("Section: x11"));
        assert!(control.contains("Priority: optional"));
        let desktop = mc.desktop_file(&test_config());
        assert!(desktop.contains("Name=hola_amigos"));
        assert!(desktop.contains("StartupNotify=true"));
    }
}
