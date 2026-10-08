use std::path::Path;
use std::process::Command;

use fastforge_core::{
    AppPackager, PackageConfig, PackageError, PackageResult, Platform, Variables,
};
use serde::Deserialize;

use super::common::{
    FormatVariables, Person, RawPackaging, deb_architecture, desktop_categories, desktop_list,
    load_make_config, make_executable, render_desktop_entry, var,
};
use super::staging::{self, Contents, Layout};

/// Builds a Debian `.deb` package using `dpkg-deb`, mirroring
/// Dart's `AppPackageMakerDeb`.
///
/// The package root is staged by [`staging::stage`] (bundle in
/// `/opt/<binary>`, `/usr/bin/<binary>` link, desktop entry, icons,
/// metainfo, `files/` overlays). Raw files in `.fastforge/packaging/linux/deb/`
/// (`control`, maintainer scripts, `conffiles`, `triggers`, ...) are rendered
/// with fastforge's variables and win over what would be generated from
/// `linux/packaging/deb/make_config.yaml` (same schema as Dart's
/// `MakeDebConfig`) or defaults. `Installed-Size`, `md5sums` and the
/// `/etc` entries of `conffiles` are filled in automatically.
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

    /// Renders the default `DEBIAN/control`. Project metadata
    /// (`APP_MAINTAINER`, `APP_DESCRIPTION`, `APP_HOMEPAGE`) fills what
    /// `make_config.yaml` leaves out; empty lists are omitted.
    fn control_file(&self, config: &PackageConfig, variables: &Variables) -> String {
        let join = |v: &Option<Vec<String>>| -> Option<String> {
            v.as_ref().filter(|v| !v.is_empty()).map(|v| v.join(", "))
        };
        // `Description` is mandatory: the description, else the app's name.
        let description = var(variables, "APP_DESCRIPTION")
            .or_else(|| var(variables, "APP_DISPLAY_NAME"))
            .unwrap_or_else(|| config.app_name.clone());

        let entries: Vec<(&str, Option<String>)> = vec![
            (
                "Package",
                Some(
                    var(variables, "PACKAGE_NAME")
                        .unwrap_or_else(|| deb_package_name(&config.app_binary_name)),
                ),
            ),
            (
                "Version",
                Some(
                    var(variables, "PACKAGE_VERSION")
                        .unwrap_or_else(|| deb_version(&config.app_version)),
                ),
            ),
            ("Architecture", Some(deb_architecture().to_string())),
            (
                "Maintainer",
                self.maintainer
                    .as_ref()
                    .map(Person::formatted)
                    .or_else(|| var(variables, "APP_MAINTAINER")),
            ),
            (
                "Uploaders",
                self.co_authors.as_ref().filter(|v| !v.is_empty()).map(|v| {
                    v.iter()
                        .map(Person::formatted)
                        .collect::<Vec<_>>()
                        .join(", ")
                }),
            ),
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
            (
                "Essential",
                self.essential
                    .map(|e| (if e { "yes" } else { "no" }).to_string()),
            ),
            ("Installed-Size", self.installed_size.map(|s| s.to_string())),
            ("Homepage", var(variables, "APP_HOMEPAGE")),
            ("Pre-Depends", join(&self.pre_dependencies)),
            // A Flutter app needs GTK; an empty list opts out.
            (
                "Depends",
                match &self.dependencies {
                    None => Some("libgtk-3-0".to_string()),
                    deps => join(deps),
                },
            ),
            ("Recommends", join(&self.recommended_dependencies)),
            ("Suggests", join(&self.suggested_dependencies)),
            ("Enhances", join(&self.enhances)),
            ("Breaks", join(&self.breaks)),
            ("Conflicts", join(&self.conflicts)),
            ("Provides", join(&self.provides)),
            ("Replaces", join(&self.replaces)),
            ("Build-Depends", join(&self.build_dependencies)),
            ("Build-Depends-Indep", join(&self.build_dependencies_indep)),
            ("Description", Some(description)),
        ];

        let mut out = String::new();
        for (key, value) in entries {
            if let Some(value) = value {
                out.push_str(&format!("{}: {}\n", key, value));
            }
        }
        out
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
            // Only written when `startup_notify` is configured (like Dart).
            ("StartupNotify", self.startup_notify.map(|b| b.to_string())),
            ("StartupWMClass", self.startup_wm_class.clone()),
        ])
    }

    /// `postinst` from `postinstall_scripts`, `None` when there are none (the
    /// `/usr/bin` link is part of the package, not created by a script).
    fn postinst(&self) -> Option<String> {
        maintainer_script(&self.postinstall_scripts)
    }

    /// `postrm` from `postuninstall_scripts`, `None` when there are none.
    fn postrm(&self) -> Option<String> {
        maintainer_script(&self.postuninstall_scripts)
    }
}

fn maintainer_script(lines: &Option<Vec<String>>) -> Option<String> {
    let lines = lines.as_ref().filter(|lines| !lines.is_empty())?;
    Some(format!("#!/bin/sh\n{}\nexit 0\n", lines.join("\n")))
}

/// Adds `Installed-Size` (KiB) to a control file that does not set it, before
/// `Description` (whose continuation lines must stay last). Also makes sure
/// the file ends with a newline, as dpkg requires.
fn with_installed_size(control: &str, kib: u64) -> String {
    let mut control = control.trim_end_matches('\n').to_string();
    control.push('\n');
    let has_field = control
        .lines()
        .any(|line| line.to_ascii_lowercase().starts_with("installed-size:"));
    if has_field {
        return control;
    }
    let field = format!("Installed-Size: {}\n", kib);
    match control.find("\nDescription:") {
        Some(pos) => {
            control.insert_str(pos + 1, &field);
            control
        }
        None if control.starts_with("Description:") => format!("{}{}", field, control),
        None => control + &field,
    }
}

/// `conffiles`: the raw list plus every file under `/etc`, without repeats.
fn merge_conffiles(raw: Option<&str>, etc_files: &[String]) -> String {
    let mut entries: Vec<String> = raw
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    for file in etc_files {
        if !entries.contains(file) {
            entries.push(file.clone());
        }
    }
    entries.iter().map(|e| format!("{}\n", e)).collect()
}

/// The Debian version of an app version: a pre-release's `-` (the revision
/// separator in Debian) becomes `~`, which sorts before the release
/// (`1.2.3-beta.1+4` -> `1.2.3~beta.1+4`).
fn deb_version(app_version: &str) -> String {
    match app_version.split_once('+') {
        Some((name, build)) => format!("{}+{}", name.replace('-', "~"), build),
        None => app_version.replace('-', "~"),
    }
}

/// The default Debian package name for a binary: lowercased, with characters
/// Debian does not allow (anything but `a-z0-9+-.`, e.g. `_`) replaced by `-`.
fn deb_package_name(binary_name: &str) -> String {
    binary_name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect()
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
        let output_file = config.output_file();
        let binary_name = &config.app_binary_name;
        let layout = Layout::system(binary_name);
        let raw = RawPackaging::load(
            config,
            "deb",
            FormatVariables {
                package_name: make_config.package_name.clone(),
                default_package_name: deb_package_name(binary_name),
                package_version: deb_version(&config.app_version),
                package_arch: deb_architecture().to_string(),
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

        let debian_dir = pkg_dir.join("DEBIAN");
        std::fs::create_dir_all(&debian_dir)?;

        // control, with the installed size filled in.
        let control = match raw.file(&["control"]) {
            Some(src) => raw.render(&src)?,
            None => make_config.control_file(config, raw.variables()),
        };
        let (_, kib) = staging::installed_size(&pkg_dir, &["DEBIAN"])?;
        std::fs::write(
            debian_dir.join("control"),
            with_installed_size(&control, kib),
        )?;

        // Maintainer scripts: raw, else generated from make_config.yaml.
        for (name, generated) in [
            ("postinst", make_config.postinst()),
            ("postrm", make_config.postrm()),
            ("preinst", None),
            ("prerm", None),
            ("config", None),
        ] {
            let content = match raw.file(&[name]) {
                Some(src) => Some(raw.render(&src)?),
                None => generated,
            };
            if let Some(content) = content {
                let dest = debian_dir.join(name);
                std::fs::write(&dest, content)?;
                make_executable(&dest)?;
            }
        }

        // Other control files are only written when provided.
        for name in ["triggers", "templates", "shlibs", "symbols"] {
            if let Some(src) = raw.file(&[name]) {
                std::fs::write(debian_dir.join(name), raw.render(&src)?)?;
            }
        }

        // conffiles (the raw list plus /etc) and md5sums.
        let raw_conffiles = match raw.file(&["conffiles"]) {
            Some(src) => Some(raw.render(&src)?),
            None => None,
        };
        let conffiles =
            merge_conffiles(raw_conffiles.as_deref(), &staging::config_files(&pkg_dir)?);
        if !conffiles.is_empty() {
            std::fs::write(debian_dir.join("conffiles"), conffiles)?;
        }
        std::fs::write(
            debian_dir.join("md5sums"),
            staging::md5sums(&pkg_dir, &["DEBIAN"])?,
        )?;

        let mut cmd = Command::new("dpkg-deb");
        cmd.args([
            "--build",
            "--root-owner-group",
            &pkg_dir.display().to_string(),
            &output_file.display().to_string(),
        ]);
        raw.apply_env(&mut cmd);
        run(&mut cmd)?;

        std::fs::remove_dir_all(&pkg_dir).ok();
        config.resolve_result(output_file)
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
            environment: Default::default(),
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

    fn variables(entries: &[(&str, &str)]) -> Variables {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn control_file_contains_all_fields() {
        let control = full_make_config().control_file(&test_config(), &Variables::new());
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
        let desktop = full_make_config().desktop_file(&test_config(), "hola_amigos");
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
    fn scripts_only_run_configured_lines() {
        let mc = full_make_config();
        let postinst = mc.postinst().unwrap();
        assert_eq!(postinst, "#!/bin/sh\necho Installed\nexit 0\n");
        assert!(!postinst.contains("/usr/bin"));
        assert_eq!(mc.postrm().unwrap(), "#!/bin/sh\necho Removed\nexit 0\n");
        assert_eq!(DebMakeConfig::default().postinst(), None);
        assert_eq!(DebMakeConfig::default().postrm(), None);
    }

    #[test]
    fn installed_size_is_added_before_the_description() {
        let control = "Package: x\nVersion: 1\nDescription: short\n long\n";
        assert_eq!(
            with_installed_size(control, 42),
            "Package: x\nVersion: 1\nInstalled-Size: 42\nDescription: short\n long\n"
        );
        assert_eq!(
            with_installed_size("Package: x", 7),
            "Package: x\nInstalled-Size: 7\n"
        );
        let set = "Package: x\nInstalled-Size: 1\n";
        assert_eq!(with_installed_size(set, 42), set);
    }

    #[test]
    fn conffiles_merge_raw_entries_and_etc() {
        let etc = vec!["/etc/x/a.conf".to_string(), "/etc/x/b.conf".to_string()];
        assert_eq!(
            merge_conffiles(Some("/etc/x/b.conf\n/etc/y.conf\n"), &etc),
            "/etc/x/b.conf\n/etc/y.conf\n/etc/x/a.conf\n"
        );
        assert_eq!(merge_conffiles(None, &[]), "");
    }

    #[test]
    fn versions_follow_debian_rules() {
        assert_eq!(deb_version("1.2.3+4"), "1.2.3+4");
        assert_eq!(deb_version("1.2.3-beta.1+4"), "1.2.3~beta.1+4");
        assert_eq!(deb_version("1.2.3-rc.1"), "1.2.3~rc.1");
    }

    #[test]
    fn default_package_name_is_a_valid_debian_name() {
        assert_eq!(deb_package_name("hola_amigos"), "hola-amigos");
        assert_eq!(deb_package_name("Hello.World+2"), "hello.world+2");
    }

    #[test]
    fn defaults_without_make_config() {
        let mc = DebMakeConfig::default();
        let control = mc.control_file(&test_config(), &Variables::new());
        assert!(control.contains("Package: hola-amigos"));
        assert!(control.contains("Depends: libgtk-3-0\n"));
        assert!(control.contains("Section: x11"));
        assert!(control.contains("Priority: optional"));
        // Description is mandatory: the app name stands in for it.
        assert!(control.ends_with("Description: hola_amigos\n"));
        assert!(!control.contains("Maintainer"));

        // Project metadata fills the gaps.
        let control = mc.control_file(
            &test_config(),
            &variables(&[
                ("APP_MAINTAINER", "Jane <jane@example.com>"),
                ("APP_DESCRIPTION", "A demo"),
                ("APP_HOMEPAGE", "https://example.com"),
            ]),
        );
        assert!(control.contains("Maintainer: Jane <jane@example.com>\n"));
        assert!(control.contains("Homepage: https://example.com\n"));
        assert!(control.ends_with("Description: A demo\n"));
        let desktop = mc.desktop_file(&test_config(), "hola_amigos");
        assert!(desktop.contains("Name=hola_amigos"));
        // Dart omits StartupNotify when `startup_notify` is not configured.
        assert!(!desktop.contains("StartupNotify"));
        assert!(desktop.contains("Categories=Utility;"));
    }

    #[test]
    fn empty_lists_are_omitted() {
        let mc: DebMakeConfig =
            serde_yaml::from_str("dependencies: []\nco_authors: []\nstartup_notify: false\n")
                .unwrap();
        let control = mc.control_file(&test_config(), &Variables::new());
        assert!(!control.contains("Depends"));
        assert!(!control.contains("Uploaders"));
        assert!(!control.contains("Recommends"));
        let desktop = mc.desktop_file(&test_config(), "hola_amigos");
        assert!(desktop.contains("StartupNotify=false"));
    }
}
