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
    fn control_file(
        &self,
        config: &PackageConfig,
        variables: &Variables,
        shlib_depends: Option<&str>,
    ) -> String {
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
            // A Flutter app needs GTK; an empty list opts out. The shared
            // libraries the binaries link against are added after the
            // configured dependencies.
            (
                "Depends",
                match &self.dependencies {
                    None => Some(merge_depends("libgtk-3-0", shlib_depends)),
                    deps => join(deps).map(|deps| merge_depends(&deps, shlib_depends)),
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

    /// Whether the generated control file wants `dpkg-shlibdeps`: an empty
    /// `dependencies` list opts out of every automatic dependency.
    fn wants_shlib_depends(&self) -> bool {
        self.dependencies
            .as_ref()
            .is_none_or(|deps| !deps.is_empty())
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

    /// Repair a package-owned launcher removed by Dart's old postrm during
    /// upgrade, before running the configured installation commands.
    fn postinst(&self, launcher_repair: Option<String>) -> Option<String> {
        let mut lines: Vec<String> = launcher_repair.into_iter().collect();
        lines.extend(self.postinstall_scripts.iter().flatten().cloned());
        maintainer_script(&Some(lines))
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

/// Only symlinks shipped in the final payload need repair. In particular, a
/// launcher replaced with a regular file by an overlay must stay untouched.
fn launcher_repair(root: &Path, binary_name: &str) -> Result<Option<String>, PackageError> {
    let link = format!("/usr/bin/{binary_name}");
    let staged_link = root.join(link.trim_start_matches('/'));
    let metadata = match std::fs::symlink_metadata(&staged_link) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_symlink() {
        return Ok(None);
    }
    let target = std::fs::read_link(staged_link)?;
    let target = target
        .to_str()
        .ok_or_else(|| PackageError::General("DEB launcher target must be valid UTF-8".into()))?;
    Ok(Some(restore_launcher(&link, target)))
}

fn restore_launcher(link: &str, target: &str) -> String {
    let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
    let link = quote(link);
    // dpkg runs old postrm after unpacking the new payload. Restore only a
    // missing entry during configure, without replacing an existing file or
    // dangling symlink. The link remains owned and removed by dpkg.
    format!(
        "if [ \"${{1:-}}\" = configure ] && [ ! -e {link} ] && [ ! -L {link} ]; then\n    ln -s -- {} {link} || exit \"$?\"\nfi",
        quote(target),
    )
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
        let pkg_dir = config.packaging_dir()?;
        let output_file = config.output_file()?;
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
        // dpkg-deb rejects a control directory outside 0755..=0775, which a
        // permissive umask (such as 000 in some containers) would create.
        make_executable(&debian_dir)?;

        // control, with the installed size filled in. Shared-library
        // dependencies come from dpkg-shlibdeps: added to a generated
        // control file when it is available, and substituted for
        // `${shlibs:Depends}` in a raw one, where it is required.
        let control = match raw.file(&["control"]) {
            Some(src) => {
                let control = raw.render(&src)?;
                if control.contains(SHLIBS_DEPENDS) {
                    let depends = shlib_depends(&pkg_dir, &var_or(&raw, "PACKAGE_NAME"))?;
                    substitute_shlibs_depends(&control, &depends)
                } else {
                    control
                }
            }
            None => {
                let depends = if make_config.wants_shlib_depends() {
                    shlib_depends(&pkg_dir, &var_or(&raw, "PACKAGE_NAME"))
                        .map_err(|err| {
                            eprintln!(
                                "Warning: shared-library dependencies were not added: {}",
                                err
                            )
                        })
                        .ok()
                } else {
                    None
                };
                make_config.control_file(config, raw.variables(), depends.as_deref())
            }
        };
        let (_, kib) = staging::installed_size(&pkg_dir, &["DEBIAN"])?;
        std::fs::write(
            debian_dir.join("control"),
            with_installed_size(&control, kib),
        )?;

        // Maintainer scripts: raw, else generated from make_config.yaml.
        for name in ["postinst", "postrm", "preinst", "prerm", "config"] {
            let content = match raw.file(&[name]) {
                Some(src) => Some(raw.render(&src)?),
                None => match name {
                    "postinst" => make_config.postinst(launcher_repair(&pkg_dir, binary_name)?),
                    "postrm" => make_config.postrm(),
                    _ => None,
                },
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
            // xz instead of dpkg-deb's default zstd, which older dpkg
            // releases (Debian before 12, deepin, UOS) cannot unpack.
            "-Zxz",
            &pkg_dir.display().to_string(),
            &output_file.display().to_string(),
        ]);
        raw.apply_env(&mut cmd);
        run(&mut cmd)?;

        std::fs::remove_dir_all(&pkg_dir).ok();
        config.resolve_result(output_file)
    }
}

const SHLIBS_DEPENDS: &str = "${shlibs:Depends}";

fn var_or(raw: &RawPackaging, key: &str) -> String {
    var(raw.variables(), key).unwrap_or_else(|| "package".to_string())
}

/// The package names of a `Depends` value, such as `libc6` in
/// `libc6 (>= 2.34)`; an alternative (`a | b`) is named by its first.
fn depends_names(depends: &str) -> Vec<String> {
    depends
        .split(',')
        .filter_map(|entry| {
            entry
                .split(['|', '(', ' '])
                .map(str::trim)
                .find(|name| !name.is_empty())
                .map(|name| name.split(':').next().unwrap_or(name).to_string())
        })
        .collect()
}

/// `configured`, followed by every `shlibs` entry for a package it does not
/// already name, so a configured version constraint wins.
fn merge_depends(configured: &str, shlibs: Option<&str>) -> String {
    let named = depends_names(configured);
    let mut entries: Vec<String> = configured
        .split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(String::from)
        .collect();
    for entry in shlibs.unwrap_or_default().split(',').map(str::trim) {
        if entry.is_empty() {
            continue;
        }
        if depends_names(entry)
            .first()
            .is_some_and(|name| !named.contains(name))
        {
            entries.push(entry.to_string());
        }
    }
    entries.join(", ")
}

/// Replaces `${shlibs:Depends}` in a raw control file, dropping the
/// separator around it when there is nothing to add.
fn substitute_shlibs_depends(control: &str, depends: &str) -> String {
    if !depends.is_empty() {
        return control.replace(SHLIBS_DEPENDS, depends);
    }
    control
        .lines()
        .filter_map(|line| {
            if !line.contains(SHLIBS_DEPENDS) {
                return Some(line.to_string());
            }
            let (key, value) = line.split_once(':')?;
            let rest = value
                .replace(SHLIBS_DEPENDS, "")
                .split(',')
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            (!rest.is_empty()).then(|| format!("{}: {}", key, rest))
        })
        .map(|line| line + "\n")
        .collect()
}

/// Whether `path` is an ELF object, which dpkg-shlibdeps can analyze.
fn is_elf(path: &Path) -> bool {
    use std::io::Read;
    let mut magic = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut magic))
        .is_ok()
        && magic == *b"\x7fELF"
}

fn elf_files(dir: &Path, root: &Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if path != root.join("DEBIAN") {
                elf_files(&path, root, out)?;
            }
        } else if file_type.is_file() && is_elf(&path) {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

/// The `Depends` value `dpkg-shlibdeps` derives from the shared libraries
/// the package's binaries link against, as debhelper's `${shlibs:Depends}`
/// would. Libraries the package ships itself, such as the Flutter engine,
/// are recognized as its own. Requires `dpkg-shlibdeps` (`dpkg-dev`) and
/// the host's libraries installed through dpkg, so it works on Debian and
/// Ubuntu hosts; elsewhere it fails.
fn shlib_depends(pkg_dir: &Path, package_name: &str) -> Result<String, PackageError> {
    let mut files = Vec::new();
    elf_files(pkg_dir, pkg_dir, &mut files)?;
    if files.is_empty() {
        return Ok(String::new());
    }
    files.sort();

    // dpkg-shlibdeps expects a source tree: debian/control naming the
    // package, and the package root at debian/<package>.
    let work = tempfile_dir()?;
    let debian = work.join("debian");
    std::fs::create_dir_all(&debian)?;
    std::fs::write(
        debian.join("control"),
        format!(
            "Source: {0}\n\nPackage: {0}\nArchitecture: any\n",
            package_name
        ),
    )?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(std::fs::canonicalize(pkg_dir)?, debian.join(package_name))?;

    let mut cmd = Command::new("dpkg-shlibdeps");
    cmd.current_dir(&work)
        .args(["-O", "--ignore-missing-info"])
        .args(
            files
                .iter()
                .map(|file| Path::new("debian").join(package_name).join(file)),
        );
    let output = cmd.output();
    std::fs::remove_dir_all(&work).ok();
    let output = output.map_err(|e| {
        PackageError::MissingTool(format!("dpkg-shlibdeps: {} (install dpkg-dev)", e))
    })?;
    if !output.status.success() {
        return Err(PackageError::CommandFailed {
            command: "dpkg-shlibdeps".into(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("shlibs:Depends="))
        .unwrap_or_default()
        .trim()
        .to_string())
}

/// A fresh scratch directory under the system temp directory.
fn tempfile_dir() -> Result<std::path::PathBuf, PackageError> {
    let dir = std::env::temp_dir().join(format!(
        "fastforge-shlibdeps-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
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
    fn shared_library_dependencies_follow_the_configured_ones() {
        let shlibs = "libc6 (>= 2.34), libgtk-3-0 (>= 3.21.4), zlib1g (>= 1:1.1.4)";
        assert_eq!(
            merge_depends("libgtk-3-0", Some(shlibs)),
            "libgtk-3-0, libc6 (>= 2.34), zlib1g (>= 1:1.1.4)"
        );
        assert_eq!(
            merge_depends("libmpv2 | libmpv1, libc6 (>= 2.36)", Some(shlibs)),
            "libmpv2 | libmpv1, libc6 (>= 2.36), libgtk-3-0 (>= 3.21.4), zlib1g (>= 1:1.1.4)"
        );
        assert_eq!(merge_depends("libgtk-3-0", None), "libgtk-3-0");

        let mc = DebMakeConfig::default();
        let control = mc.control_file(&test_config(), &Variables::new(), Some(shlibs));
        assert!(control.contains("Depends: libgtk-3-0, libc6 (>= 2.34), zlib1g (>= 1:1.1.4)\n"));
        let opted_out: DebMakeConfig = serde_yaml::from_str("dependencies: []").unwrap();
        assert!(!opted_out.wants_shlib_depends());
        assert!(mc.wants_shlib_depends());
    }

    /// Runs dpkg-shlibdeps on a staged package root named by
    /// `FASTFORGE_SHLIBDEPS_ROOT`; needs a Debian or Ubuntu host.
    #[test]
    #[ignore]
    fn shlib_depends_on_a_debian_host() {
        let root = std::env::var("FASTFORGE_SHLIBDEPS_ROOT").unwrap();
        let depends = shlib_depends(Path::new(&root), "hello-world").unwrap();
        println!("{depends}");
        assert!(depends.contains("libc6") && depends.contains("libgtk-3-0"));
    }

    #[test]
    fn raw_control_substitutes_shlibs_depends() {
        let control = "Package: demo\nDepends: ${shlibs:Depends}, libmpv2\nDescription: d\n";
        assert_eq!(
            substitute_shlibs_depends(control, "libc6 (>= 2.34)"),
            "Package: demo\nDepends: libc6 (>= 2.34), libmpv2\nDescription: d\n"
        );
        assert_eq!(
            substitute_shlibs_depends(control, ""),
            "Package: demo\nDepends: libmpv2\nDescription: d\n"
        );
        assert_eq!(
            substitute_shlibs_depends("Depends: ${shlibs:Depends}\nDescription: d\n", ""),
            "Description: d\n"
        );
    }

    #[test]
    fn control_file_contains_all_fields() {
        let control = full_make_config().control_file(&test_config(), &Variables::new(), None);
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
        let postinst = mc.postinst(None).unwrap();
        assert_eq!(postinst, "#!/bin/sh\necho Installed\nexit 0\n");
        assert!(!postinst.contains("/usr/bin"));
        assert_eq!(mc.postrm().unwrap(), "#!/bin/sh\necho Removed\nexit 0\n");
        assert_eq!(DebMakeConfig::default().postinst(None), None);
        assert_eq!(DebMakeConfig::default().postrm(), None);
    }

    #[test]
    #[cfg(unix)]
    fn postinst_restores_only_missing_launchers_during_configure() {
        let tmp = tempfile::tempdir().unwrap();
        // Both paths exercise shell quoting; a dangling symlink is valid.
        let link = tmp.path().join("launcher's $entry");
        let target = "../bundle's $executable";
        let marker = tmp.path().join("configured");
        let mc = DebMakeConfig {
            postinstall_scripts: Some(vec!["test -L \"$LAUNCHER\" && touch \"$MARKER\"".into()]),
            ..Default::default()
        };
        let script = tmp.path().join("postinst");
        std::fs::write(
            &script,
            mc.postinst(Some(restore_launcher(link.to_str().unwrap(), target)))
                .unwrap(),
        )
        .unwrap();
        let execute = |argument: &str| {
            Command::new("sh")
                .arg(&script)
                .arg(argument)
                .env("LAUNCHER", &link)
                .env("MARKER", &marker)
                .status()
                .unwrap()
        };
        assert!(execute("abort-upgrade").success());
        assert!(link.symlink_metadata().is_err());
        assert!(!marker.exists());
        assert!(execute("configure").success());
        assert_eq!(std::fs::read_link(&link).unwrap(), Path::new(target));
        assert!(marker.exists(), "configured commands must run after repair");
        assert!(execute("configure").success(), "repair must be idempotent");
        // Keep a replacement dangling symlink, and a regular file too.
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("custom-target", &link).unwrap();
        assert!(execute("configure").success());
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            Path::new("custom-target")
        );
        std::fs::remove_file(&link).unwrap();
        std::fs::write(&link, "custom launcher").unwrap();
        assert!(execute("configure").success());
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "custom launcher");
    }

    #[test]
    #[cfg(unix)]
    fn launcher_repair_uses_the_final_overlay_target() {
        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join("usr/bin/demo");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        assert!(launcher_repair(tmp.path(), "demo").unwrap().is_none());
        std::fs::write(&link, "custom launcher").unwrap();
        assert!(launcher_repair(tmp.path(), "demo").unwrap().is_none());
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("../../opt/custom/demo", &link).unwrap();
        assert_eq!(
            launcher_repair(tmp.path(), "demo").unwrap(),
            Some(restore_launcher("/usr/bin/demo", "../../opt/custom/demo")),
        );
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
        let control = mc.control_file(&test_config(), &Variables::new(), None);
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
            None,
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
        let control = mc.control_file(&test_config(), &Variables::new(), None);
        assert!(!control.contains("Depends"));
        assert!(!control.contains("Uploaders"));
        assert!(!control.contains("Recommends"));
        let desktop = mc.desktop_file(&test_config(), "hola_amigos");
        assert!(desktop.contains("StartupNotify=false"));
    }
}
