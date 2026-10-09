//! Helpers shared by the Linux packagers (deb / rpm / pacman / appimage),
//! mirroring the pieces of Dart's `MakeLinuxPackageConfig` and maker
//! implementations that are common across formats.

use std::path::{Path, PathBuf};
use std::process::Command;

use fastforge_core::{
    PackageConfig, PackageError, ProjectSettings, Variables, environment_variables,
    render_variables,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;

/// Loads a maker's `make_config.yaml`. Returns `Ok(None)` when the file does
/// not exist so callers can fall back to defaults (Dart requires the file for
/// deb/rpm/pacman/appimage; the Rust CLI keeps working without one, using the
/// same defaults as before).
pub(crate) fn load_make_config<T: DeserializeOwned>(
    path: &Path,
) -> Result<Option<T>, PackageError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackageError::General(format!("Failed to read {}: {}", path.display(), e)))?;
    let value: T = serde_yaml::from_str(&content)
        .map_err(|e| PackageError::General(format!("Failed to parse {}: {}", path.display(), e)))?;
    Ok(Some(value))
}

/// `name`/`email` pair used by `maintainer:` and `co_authors:` entries.
#[derive(Debug, Clone, Deserialize)]
pub struct Person {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
}

impl Person {
    /// Renders as `Name <email>`, matching Dart's formatting.
    pub fn formatted(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}

/// Machine architecture from `uname -m` (e.g. `x86_64`, `aarch64`).
pub(crate) fn uname_machine() -> String {
    std::process::Command::new("uname")
        .arg("-m")
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::env::consts::ARCH.to_string())
}

/// Debian-style architecture name, mirroring Dart's `_getArchitecture`.
pub(crate) fn deb_architecture() -> &'static str {
    if uname_machine() == "aarch64" {
        "arm64"
    } else {
        "amd64"
    }
}

/// A variable's value, `None` when unset or empty.
pub(crate) fn var(variables: &Variables, key: &str) -> Option<String> {
    variables
        .get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
}

/// Renders a `[Desktop Entry]` file from `(key, Option<value>)` pairs,
/// skipping entries whose value is `None`.
pub(crate) fn render_desktop_entry(entries: &[(&str, Option<String>)]) -> String {
    let mut lines = vec!["[Desktop Entry]".to_string()];
    for (key, value) in entries {
        if let Some(value) = value {
            lines.push(format!("{}={}", key, value));
        }
    }
    lines.join("\n")
}

/// The `Categories` of a generated desktop entry: the configured ones, else
/// `Utility;` (the spec expects a main category, and appimagetool requires
/// one).
pub(crate) fn desktop_categories(values: &Option<Vec<String>>) -> Option<String> {
    desktop_list(values).or_else(|| Some("Utility;".to_string()))
}

/// Joins list values as `a;b;c;` (freedesktop list syntax); `None` when empty.
pub(crate) fn desktop_list(values: &Option<Vec<String>>) -> Option<String> {
    values
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| format!("{};", v.join(";")))
}

/// Format-specific facts a packager contributes to the variable set.
pub(crate) struct FormatVariables<'a> {
    /// `package_name` from `make_config.yaml`; else `project.package_name`,
    /// else `default_package_name`.
    pub package_name: Option<String>,
    pub default_package_name: String,
    /// The version in the format's syntax (`PACKAGE_VERSION`).
    pub package_version: String,
    pub package_arch: String,
    /// Where the bundle is installed (`/opt/<binary>`); `None` for AppImage.
    pub install_dir: Option<String>,
    /// `display_name` from `make_config.yaml`, used when `project:` has none.
    pub display_name: Option<String>,
    /// The package root (see [`super::staging`]): the DEB or Pacman
    /// package's root, the tree an RPM spec copies, or the AppDir.
    pub packaging_dir: &'a Path,
    pub output_file: &'a Path,
    pub extra: Vec<(&'static str, String)>,
}

/// Where the raw Linux packaging files live: one directory per format, plus
/// [`SHARED_DIR`] for what every format uses.
pub(crate) const RAW_PACKAGING_DIR: &str = ".fastforge/packaging/linux";

/// The directory, next to the format directories, holding what every format
/// shares.
pub(crate) const SHARED_DIR: &str = "shared";

/// A desktop entry template: one `*.desktop` file, under any name; installed
/// as `${APP_ID}.desktop`.
pub(crate) const DESKTOP_TEMPLATE: &[&str] = &[".desktop"];

/// An AppStream metainfo template: one `*.metainfo.xml` (or the older
/// `*.appdata.xml`) file, under any name; installed as
/// `usr/share/metainfo/${APP_ID}.metainfo.xml`.
pub(crate) const METAINFO_TEMPLATE: &[&str] = &[".metainfo.xml", ".appdata.xml"];

/// `.fastforge/packaging/linux/shared/` for a format directory.
fn shared_dir(format_dir: &Path) -> Option<PathBuf> {
    format_dir.parent().map(|dir| dir.join(SHARED_DIR))
}

/// The raw packaging files of one Linux format, read from
/// `.fastforge/packaging/linux/<format>/` (and `shared/`), plus the variables
/// they are rendered with.
///
/// A raw file replaces the file fastforge would otherwise generate (from
/// `make_config.yaml` or defaults), one file at a time; `files/` (shared,
/// then the format's own) is an overlay copied into the package root. Text
/// files and file names are rendered with [`render_variables`]; binary files
/// are copied as they are.
pub(crate) struct RawPackaging {
    dir: PathBuf,
    settings: ProjectSettings,
    variables: Variables,
}

impl RawPackaging {
    pub fn load(
        config: &PackageConfig,
        format: &str,
        facts: FormatVariables,
    ) -> Result<Self, PackageError> {
        Self::load_from(
            Path::new(RAW_PACKAGING_DIR).join(format),
            ProjectSettings::load()?,
            config,
            facts,
        )
    }

    pub fn load_from(
        dir: PathBuf,
        settings: ProjectSettings,
        config: &PackageConfig,
        facts: FormatVariables,
    ) -> Result<Self, PackageError> {
        let mut variables = config.package_variables(&settings);
        let absolute = |path: &Path| {
            std::path::absolute(path)
                .unwrap_or_else(|_| path.to_path_buf())
                .display()
                .to_string()
        };
        let mut set = |key: &str, value: String| {
            variables.insert(key.to_string(), value);
        };
        if settings.display_name.is_none()
            && let Some(name) = facts.display_name
        {
            set("APP_DISPLAY_NAME", name);
        }
        set(
            "PACKAGE_NAME",
            facts
                .package_name
                .or_else(|| settings.package_name.clone())
                .unwrap_or(facts.default_package_name),
        );
        set("PACKAGE_VERSION", facts.package_version);
        set("PACKAGE_ARCH", facts.package_arch);
        set("ARCH", machine_architecture().to_string());
        if let Some(install_dir) = facts.install_dir {
            set("INSTALL_DIR", install_dir);
        }
        set("PACKAGING_DIRECTORY", absolute(facts.packaging_dir));
        set("OUTPUT_ARTIFACT_PATH", absolute(facts.output_file));
        for (key, value) in facts.extra {
            set(key, value);
        }
        // Catch an ambiguous desktop template (several files) early.
        let raw = Self {
            dir,
            settings,
            variables,
        };
        raw.template(DESKTOP_TEMPLATE)?;
        Ok(raw)
    }

    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// `APP_ID`, which names the desktop file, the icon and the metainfo so
    /// they match the window's app ID that Wayland compositors look up.
    pub fn app_id(&self) -> &str {
        &self.variables["APP_ID"]
    }

    /// The package name (`PACKAGE_NAME`).
    pub fn package_name(&self) -> &str {
        &self.variables["PACKAGE_NAME"]
    }

    /// The icon from `make_config.yaml`, else `project.icon`.
    pub fn icon(&self, make_config_icon: Option<&String>) -> Option<String> {
        make_config_icon
            .cloned()
            .or_else(|| self.settings.icon.clone())
    }

    /// Sets the (non-empty) variables in the environment of a packaging tool.
    pub fn apply_env(&self, cmd: &mut Command) {
        cmd.envs(environment_variables(&self.variables));
    }

    /// `<dir>/<name>` when it is a file.
    pub fn file(&self, names: &[&str]) -> Option<PathBuf> {
        names
            .iter()
            .map(|name| self.dir.join(name))
            .find(|path| path.is_file())
    }

    /// The only file in the format directory with extension `ext`; an error
    /// when there are several, since fastforge could not tell which to use.
    pub fn file_with_extension(&self, ext: &str) -> Result<Option<PathBuf>, PackageError> {
        single_file_with_suffix(&self.dir, &[&format!(".{}", ext)])
    }

    /// The template ending with one of `suffixes` (see [`DESKTOP_TEMPLATE`],
    /// [`METAINFO_TEMPLATE`]), whatever its name: the format directory's,
    /// else the shared one. Several in one directory are an error, since
    /// fastforge could not tell which to use.
    pub fn template(&self, suffixes: &[&str]) -> Result<Option<PathBuf>, PackageError> {
        let own = single_file_with_suffix(&self.dir, suffixes)?;
        let shared = match shared_dir(&self.dir) {
            Some(dir) => single_file_with_suffix(&dir, suffixes)?,
            None => None,
        };
        Ok(own.or(shared))
    }

    pub fn render(&self, path: &Path) -> Result<String, PackageError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            PackageError::General(format!("Failed to read {}: {}", path.display(), e))
        })?;
        Ok(render_variables(&content, &self.variables))
    }

    /// Writes the rendered raw file `src` to `dest`. The contents of `generate`
    /// are written instead when there is no raw file.
    pub fn write_or_generate(
        &self,
        src: Option<PathBuf>,
        dest: &Path,
        generate: impl FnOnce() -> String,
    ) -> Result<(), PackageError> {
        let content = match src {
            Some(src) => self.render(&src)?,
            None => generate(),
        };
        std::fs::write(dest, content)?;
        Ok(())
    }

    /// Copies the `files/` overlays into `dest`: the one shared by every
    /// format first, then the format's own, so its files win. With
    /// `shared_top_level`, only those top-level directories of the shared
    /// overlay are copied (an AppDir has no use for `etc/`). File names and
    /// text files are rendered; permissions and symlinks are kept.
    pub fn install_overlay(
        &self,
        dest: &Path,
        shared_top_level: Option<&[&str]>,
    ) -> Result<(), PackageError> {
        if let Some(shared) = shared_dir(&self.dir).map(|dir| dir.join("files"))
            && shared.is_dir()
        {
            copy_rendered_tree(&shared, dest, &self.variables, shared_top_level)?;
        }
        let own = self.dir.join("files");
        if own.is_dir() {
            copy_rendered_tree(&own, dest, &self.variables, None)?;
        }
        Ok(())
    }
}

/// The only file directly in `dir` whose name ends with one of `suffixes`;
/// an error when there are several.
fn single_file_with_suffix(dir: &Path, suffixes: &[&str]) -> Result<Option<PathBuf>, PackageError> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            path.is_file() && suffixes.iter().any(|suffix| name.ends_with(suffix))
        })
        .collect();
    found.sort();
    if found.len() > 1 {
        let names: Vec<String> = found
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        return Err(PackageError::General(format!(
            "{} has several {} files ({}); keep only one",
            dir.display(),
            suffixes.join(" or "),
            names.join(", ")
        )));
    }
    Ok(found.pop())
}

fn copy_rendered_tree(
    src: &Path,
    dest: &Path,
    variables: &Variables,
    only: Option<&[&str]>,
) -> Result<(), PackageError> {
    std::fs::create_dir_all(dest)?;
    let mut entries: Vec<_> = std::fs::read_dir(src)?.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = render_variables(&entry.file_name().to_string_lossy(), variables);
        if only.is_some_and(|only| !only.contains(&name.as_str())) {
            continue;
        }
        let from = entry.path();
        let to = dest.join(name);
        let meta = std::fs::symlink_metadata(&from)?;
        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&from)?;
            let target = render_variables(&target.to_string_lossy(), variables);
            if to.symlink_metadata().is_ok() {
                std::fs::remove_file(&to)?;
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, &to)?;
            #[cfg(not(unix))]
            std::fs::copy(&from, &to).map(|_| ())?;
        } else if meta.is_dir() {
            copy_rendered_tree(&from, &to, variables, None)?;
        } else {
            let bytes = std::fs::read(&from)?;
            match std::str::from_utf8(&bytes) {
                Ok(text) if !bytes.contains(&0) => {
                    std::fs::write(&to, render_variables(text, variables))?
                }
                _ => std::fs::write(&to, &bytes)?,
            }
            std::fs::set_permissions(&to, meta.permissions())?;
        }
    }
    Ok(())
}

/// Marks `path` executable (`0755`), as maintainer scripts and `AppRun` must be.
pub(crate) fn make_executable(path: &Path) -> Result<(), PackageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// `x86_64` or `aarch64`, the names rpm, pacman and AppImage use.
pub(crate) fn machine_architecture() -> &'static str {
    if uname_machine() == "aarch64" {
        "aarch64"
    } else {
        "x86_64"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_skips_none() {
        let out = render_desktop_entry(&[
            ("Type", Some("Application".into())),
            ("GenericName", None),
            ("Name", Some("Demo".into())),
        ]);
        assert_eq!(out, "[Desktop Entry]\nType=Application\nName=Demo");
    }

    #[test]
    fn desktop_list_formatting() {
        assert_eq!(
            desktop_list(&Some(vec!["Music".into(), "Media".into()])),
            Some("Music;Media;".to_string())
        );
        assert_eq!(desktop_list(&Some(vec![])), None);
        assert_eq!(desktop_list(&None), None);
    }

    #[test]
    fn person_formatting() {
        let p = Person {
            name: "Gamer Boy 69".into(),
            email: "rickastley@gmail.lol".into(),
        };
        assert_eq!(p.formatted(), "Gamer Boy 69 <rickastley@gmail.lol>");
    }

    fn test_config(output: &Path) -> PackageConfig {
        PackageConfig {
            app_name: "hola_amigos".into(),
            app_binary_name: "hola-amigos".into(),
            app_version: "1.2.3+4".into(),
            build_mode: "release".into(),
            platform: fastforge_core::Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "deb".into(),
            is_installer: false,
            build_output_dir: output.join("bundle"),
            build_output_files: vec![],
            output_dir: output.to_path_buf(),
            environment: Default::default(),
        }
    }

    fn raw(dir: &Path, settings: ProjectSettings, display_name: Option<&str>) -> RawPackaging {
        try_raw(dir, settings, display_name).unwrap()
    }

    fn try_raw(
        dir: &Path,
        settings: ProjectSettings,
        display_name: Option<&str>,
    ) -> Result<RawPackaging, PackageError> {
        let config = test_config(dir);
        RawPackaging::load_from(
            dir.join("deb"),
            settings,
            &config,
            FormatVariables {
                package_name: None,
                default_package_name: "hola-amigos".into(),
                package_version: "1.2.3+4".into(),
                package_arch: "amd64".into(),
                install_dir: Some("/opt/hola-amigos".into()),
                display_name: display_name.map(str::to_string),
                packaging_dir: &dir.join("staging"),
                output_file: &dir.join("out.deb"),
                extra: vec![("RPM_RELEASE", "4".into())],
            },
        )
    }

    #[test]
    fn raw_variables_include_format_facts() {
        let tmp = tempfile::tempdir().unwrap();
        let r = raw(tmp.path(), ProjectSettings::default(), Some("Hola Amigos"));
        let v = &r.variables;
        assert_eq!(v["PACKAGE_NAME"], "hola-amigos");
        assert_eq!(v["PACKAGE_VERSION"], "1.2.3+4");
        assert_eq!(v["PACKAGE_ARCH"], "amd64");
        assert_eq!(v["INSTALL_DIR"], "/opt/hola-amigos");
        assert_eq!(v["RPM_RELEASE"], "4");
        // Without `project.app_id`, `APP_ID` falls back to the binary name.
        assert_eq!(v["APP_ID"], "hola-amigos");
        // make_config's display_name is used when project: has none ...
        assert_eq!(v["APP_DISPLAY_NAME"], "Hola Amigos");
        assert!(v["PACKAGING_DIRECTORY"].ends_with("staging"));
        assert!(Path::new(&v["OUTPUT_ARTIFACT_PATH"]).is_absolute());

        // ... and project.display_name wins over it.
        let settings = ProjectSettings {
            display_name: Some("Project Name".into()),
            package_name: Some("hola".into()),
            icon: Some("assets/icon.png".into()),
            ..Default::default()
        };
        let r = raw(tmp.path(), settings, Some("Hola Amigos"));
        assert_eq!(r.variables["APP_DISPLAY_NAME"], "Project Name");
        // project.package_name replaces the format's default.
        assert_eq!(r.package_name(), "hola");
        assert_eq!(r.icon(None).as_deref(), Some("assets/icon.png"));
        assert_eq!(
            r.icon(Some(&"make.png".to_string())).as_deref(),
            Some("make.png")
        );
    }

    #[test]
    fn raw_files_are_rendered_or_generated() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("deb");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("control"),
            "Package: ${PACKAGE_NAME}\nVersion: ${APP_VERSION}\nArchitecture: ${PACKAGE_ARCH}\n",
        )
        .unwrap();
        let r = raw(tmp.path(), ProjectSettings::default(), None);

        let dest = tmp.path().join("control.out");
        r.write_or_generate(r.file(&["control"]), &dest, || "generated".into())
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&dest).unwrap(),
            "Package: hola-amigos\nVersion: 1.2.3+4\nArchitecture: amd64\n"
        );

        let dest = tmp.path().join("postinst.out");
        r.write_or_generate(r.file(&["postinst"]), &dest, || "generated".into())
            .unwrap();
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "generated");
    }

    #[test]
    fn file_with_extension_requires_a_single_match() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("deb");
        std::fs::create_dir_all(&dir).unwrap();
        let r = raw(tmp.path(), ProjectSettings::default(), None);
        assert_eq!(r.file_with_extension("desktop").unwrap(), None);
        std::fs::write(dir.join("a.desktop"), "").unwrap();
        assert_eq!(
            r.file_with_extension("desktop").unwrap(),
            Some(dir.join("a.desktop"))
        );
        std::fs::write(dir.join("b.desktop"), "").unwrap();
        assert!(r.file_with_extension("desktop").is_err());
        // A missing format directory just means "no raw files".
        let none = RawPackaging {
            dir: tmp.path().join("missing"),
            settings: ProjectSettings::default(),
            variables: Variables::new(),
        };
        assert_eq!(none.file_with_extension("spec").unwrap(), None);
        assert_eq!(none.file(&["control"]), None);
    }

    #[cfg(unix)]
    #[test]
    fn overlay_renders_names_and_text_and_keeps_binaries() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let files = tmp.path().join("deb/files");
        let apps = files.join("usr/share/applications");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::create_dir_all(files.join("etc")).unwrap();
        std::fs::write(
            apps.join("${APP_BINARY_NAME}.desktop"),
            "Name=${APP_DISPLAY_NAME}\nExec=${APP_BINARY_NAME} %U\nPath=${HOME}\n",
        )
        .unwrap();
        let binary = [0x89u8, b'P', b'N', b'G', 0, b'$', b'{', b'A', b'}'];
        std::fs::write(files.join("etc/logo.png"), binary).unwrap();
        let script = files.join("etc/run.sh");
        std::fs::write(&script, "#!/bin/sh\nexec ${INSTALL_DIR}/x \"$@\"\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::os::unix::fs::symlink("${INSTALL_DIR}/hola-amigos", files.join("etc/link")).unwrap();

        let r = raw(tmp.path(), ProjectSettings::default(), Some("Hola"));
        let dest = tmp.path().join("root");
        r.install_overlay(&dest, None).unwrap();

        assert_eq!(
            std::fs::read_to_string(dest.join("usr/share/applications/hola-amigos.desktop"))
                .unwrap(),
            "Name=Hola\nExec=hola-amigos %U\nPath=${HOME}\n"
        );
        assert_eq!(std::fs::read(dest.join("etc/logo.png")).unwrap(), binary);
        let run = dest.join("etc/run.sh");
        assert_eq!(
            std::fs::read_to_string(&run).unwrap(),
            "#!/bin/sh\nexec /opt/hola-amigos/x \"$@\"\n"
        );
        assert_eq!(
            std::fs::metadata(&run).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            std::fs::read_link(dest.join("etc/link")).unwrap(),
            Path::new("/opt/hola-amigos/hola-amigos")
        );
    }

    #[test]
    fn templates_are_found_under_any_name() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("deb")).unwrap();
        std::fs::create_dir_all(tmp.path().join("shared")).unwrap();
        let r = raw(tmp.path(), ProjectSettings::default(), None);
        assert_eq!(r.template(DESKTOP_TEMPLATE).unwrap(), None);

        // The shared template, whatever its name ...
        let shared = tmp.path().join("shared/${APP_ID}.desktop");
        std::fs::write(&shared, "Exec=${APP_BINARY_NAME} %U\nIcon=${APP_ID}\n").unwrap();
        let r = raw(tmp.path(), ProjectSettings::default(), None);
        assert_eq!(r.template(DESKTOP_TEMPLATE).unwrap(), Some(shared.clone()));
        assert_eq!(
            r.render(&shared).unwrap(),
            "Exec=hola-amigos %U\nIcon=hola-amigos\n"
        );

        // ... is overridden by the format's own.
        let own = tmp.path().join("deb/hola.desktop");
        std::fs::write(&own, "").unwrap();
        assert_eq!(r.template(DESKTOP_TEMPLATE).unwrap(), Some(own));

        // Metainfo: `*.metainfo.xml` or the older `*.appdata.xml`.
        let appdata = tmp.path().join("shared/hola.appdata.xml");
        std::fs::write(&appdata, "").unwrap();
        assert_eq!(r.template(METAINFO_TEMPLATE).unwrap(), Some(appdata));

        // Two templates in one directory are ambiguous.
        std::fs::write(tmp.path().join("shared/other.desktop"), "").unwrap();
        let err = try_raw(tmp.path(), ProjectSettings::default(), None)
            .err()
            .unwrap();
        assert!(err.to_string().contains("keep only one"), "{}", err);
    }

    #[test]
    fn app_id_names_the_desktop_file_and_icon() {
        let tmp = tempfile::tempdir().unwrap();
        let r = raw(tmp.path(), ProjectSettings::default(), None);
        assert_eq!(r.app_id(), "hola-amigos");
        let settings = ProjectSettings {
            app_id: Some("dev.example.hola".into()),
            ..Default::default()
        };
        let r = raw(tmp.path(), settings, None);
        assert_eq!(r.app_id(), "dev.example.hola");
        let from_cmake = ProjectSettings {
            linux_application_id: Some("dev.example.cmake".into()),
            ..Default::default()
        };
        assert_eq!(
            raw(tmp.path(), from_cmake, None).app_id(),
            "dev.example.cmake"
        );
    }

    #[test]
    fn shared_overlay_can_be_limited() {
        let tmp = tempfile::tempdir().unwrap();
        let files = tmp.path().join("shared/files");
        std::fs::create_dir_all(files.join("etc")).unwrap();
        std::fs::create_dir_all(files.join("usr/share/doc")).unwrap();
        std::fs::write(files.join("etc/app.conf"), "").unwrap();
        std::fs::write(files.join("usr/share/doc/README"), "").unwrap();
        let own = tmp.path().join("deb/files/etc");
        std::fs::create_dir_all(&own).unwrap();
        std::fs::write(own.join("own.conf"), "").unwrap();

        let r = raw(tmp.path(), ProjectSettings::default(), None);
        let dest = tmp.path().join("root");
        r.install_overlay(&dest, Some(&["usr"])).unwrap();
        assert!(dest.join("usr/share/doc/README").is_file());
        assert!(!dest.join("etc/app.conf").exists());
        // The format's own overlay is never limited.
        assert!(dest.join("etc/own.conf").is_file());
    }

    #[test]
    fn format_overlay_wins_over_the_shared_one() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("shared/files/usr/share/metainfo");
        let own = tmp.path().join("deb/files/usr/share/metainfo");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::create_dir_all(&own).unwrap();
        std::fs::write(shared.join("${APP_BINARY_NAME}.xml"), "shared ${APP_NAME}").unwrap();
        std::fs::write(shared.join("other.xml"), "shared only").unwrap();
        std::fs::write(own.join("${APP_BINARY_NAME}.xml"), "deb ${APP_NAME}").unwrap();

        std::fs::create_dir_all(tmp.path().join("shared/files/etc")).unwrap();
        std::fs::write(tmp.path().join("shared/files/etc/app.conf"), "").unwrap();

        let r = raw(tmp.path(), ProjectSettings::default(), None);
        let dest = tmp.path().join("root");
        r.install_overlay(&dest, None).unwrap();
        assert!(dest.join("etc/app.conf").is_file());
        let metainfo = dest.join("usr/share/metainfo");
        assert_eq!(
            std::fs::read_to_string(metainfo.join("hola-amigos.xml")).unwrap(),
            "deb hola_amigos"
        );
        assert_eq!(
            std::fs::read_to_string(metainfo.join("other.xml")).unwrap(),
            "shared only"
        );
    }
}
