//! Helpers shared by the Linux packagers (deb / rpm / pacman / appimage),
//! mirroring the pieces of Dart's `MakeLinuxPackageConfig` and maker
//! implementations that are common across formats.

use std::path::Path;

use fastforge_core::PackageError;
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

/// `description` / `homepage` read from the project's `pubspec.yaml`
/// (used by deb's `Description:`/`Homepage:` control fields and rpm's
/// `%description`).
#[derive(Debug, Default, Deserialize)]
pub(crate) struct PubspecMeta {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
}

pub(crate) fn load_pubspec_meta() -> PubspecMeta {
    std::fs::read_to_string("pubspec.yaml")
        .ok()
        .and_then(|content| serde_yaml::from_str(&content).ok())
        .unwrap_or_default()
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

/// Joins list values as `a;b;c;` (freedesktop list syntax); `None` when empty.
pub(crate) fn desktop_list(values: &Option<Vec<String>>) -> Option<String> {
    values
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| format!("{};", v.join(";")))
}

/// Copies the icon configured in `make_config.yaml` into
/// `usr/share/icons/hicolor/{128x128,256x256}/apps/<binary><ext>`,
/// mirroring Dart's deb/rpm maker behavior.
pub(crate) fn install_hicolor_icons(
    icon: &str,
    packaging_root: &Path,
    binary_name: &str,
) -> Result<(), PackageError> {
    let icon_path = Path::new(icon);
    if !icon_path.exists() {
        return Err(PackageError::NotFound(format!(
            "provided icon {} path wasn't found",
            icon
        )));
    }
    let ext = icon_path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for size in ["128x128", "256x256"] {
        let dir = packaging_root
            .join("usr/share/icons/hicolor")
            .join(size)
            .join("apps");
        std::fs::create_dir_all(&dir)?;
        std::fs::copy(icon_path, dir.join(format!("{}{}", binary_name, ext)))?;
    }
    Ok(())
}

/// Copies a metainfo XML into `usr/share/metainfo/<binary>.appdata.xml`
/// (double extension preserved, mirroring Dart's `path.extension(..., 2)`).
pub(crate) fn install_metainfo(
    metainfo: &str,
    packaging_root: &Path,
    binary_name: &str,
) -> Result<(), PackageError> {
    let metainfo_path = Path::new(metainfo);
    if !metainfo_path.exists() {
        return Err(PackageError::NotFound(format!(
            "Metainfo {} path wasn't found",
            metainfo
        )));
    }
    let file_name = metainfo_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    // Keep up to two extensions (e.g. `.appdata.xml`).
    let ext = {
        let parts: Vec<&str> = file_name.split('.').collect();
        match parts.len() {
            0 | 1 => String::new(),
            2 => format!(".{}", parts[1]),
            n => format!(".{}.{}", parts[n - 2], parts[n - 1]),
        }
    };
    let dir = packaging_root.join("usr/share/metainfo");
    std::fs::create_dir_all(&dir)?;
    std::fs::copy(metainfo_path, dir.join(format!("{}{}", binary_name, ext)))?;
    Ok(())
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
}
