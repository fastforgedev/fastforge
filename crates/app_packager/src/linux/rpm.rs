use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

use super::common::{
    desktop_list, load_make_config, load_pubspec_meta, render_desktop_entry, uname_machine,
};

/// Builds an RPM package using `rpmbuild`, mirroring Dart's `AppPackageMakerRPM`.
///
/// Reads `linux/packaging/rpm/make_config.yaml` when present (same schema as
/// Dart's `MakeRPMConfig`); falls back to sensible defaults otherwise.
///
/// Requires `rpmbuild` (from the `rpm-build` package) and `patchelf`.
pub struct LinuxRpmPackager;

/// Schema of `linux/packaging/rpm/make_config.yaml`, mirroring Dart's
/// `MakeRPMConfig.fromJson`.
#[derive(Debug, Default, Deserialize)]
pub struct RpmMakeConfig {
    // Desktop file
    pub display_name: Option<String>,
    pub package_name: Option<String>,
    pub startup_notify: Option<bool>,
    pub actions: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub generic_name: Option<String>,
    pub icon: Option<String>,
    pub metainfo: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub supported_mime_type: Option<Vec<String>>,

    // Spec preamble
    pub summary: Option<String>,
    pub group: Option<String>,
    pub vendor: Option<String>,
    pub packager: Option<String>,
    #[serde(alias = "packagerEmail")]
    pub packager_email: Option<String>,
    pub license: Option<String>,
    pub url: Option<String>,
    pub build_arch: Option<String>,
    pub requires: Option<Vec<String>>,
    pub build_requires: Option<Vec<String>>,

    // Spec body
    pub description: Option<String>,
    pub postun: Option<String>,
    pub postinstall_scripts: Option<Vec<String>>,
    pub postuninstall_scripts: Option<Vec<String>>,
    pub spec_macros: Option<Vec<String>>,
}

/// RPM architecture from `uname -m`, mirroring Dart's `_getArchitecture`.
fn rpm_architecture() -> String {
    if uname_machine() == "aarch64" {
        "aarch64".to_string()
    } else {
        "x86_64".to_string()
    }
}

/// Sanitizes an ELF RPATH, replacing absolute entries with `$ORIGIN` and
/// de-duplicating, mirroring Dart's `sanitizeRpmRpath`.
/// This prevents rpmbuild QA failures for build-directory RPATHs
/// (https://github.com/flutter/flutter/issues/65400).
pub fn sanitize_rpm_rpath(rpath: &str) -> String {
    let mut sanitized: Vec<String> = Vec::new();
    for entry in rpath.split(':') {
        let value = if entry.starts_with('/') {
            "$ORIGIN".to_string()
        } else {
            entry.to_string()
        };
        if !sanitized.contains(&value) {
            sanitized.push(value);
        }
    }
    sanitized.join(":")
}

impl RpmMakeConfig {
    fn load() -> Result<Self, PackageError> {
        Ok(
            load_make_config(Path::new("linux/packaging/rpm/make_config.yaml"))?
                .unwrap_or_default(),
        )
    }

    fn rpm_name(&self, config: &PackageConfig) -> String {
        self.package_name
            .clone()
            .unwrap_or_else(|| config.app_name.clone())
    }

    fn build_arch(&self) -> String {
        self.build_arch.clone().unwrap_or_else(rpm_architecture)
    }

    /// Renders the `.spec` file, mirroring Dart's `toFilesString()['SPEC']`.
    fn spec_file(&self, config: &PackageConfig) -> String {
        let meta = load_pubspec_meta();
        let build_number = config.app_version.split('+').nth(1);
        let description = self
            .description
            .clone()
            .or_else(|| meta.description.clone())
            .unwrap_or_else(|| config.app_name.clone());

        // Preamble
        let mut preamble: Vec<(&str, Option<String>)> = vec![
            ("Name", Some(self.rpm_name(config))),
            ("Version", Some(config.app_version.clone())),
            (
                "Release",
                Some(format!("{}%{{?dist}}", build_number.unwrap_or("1"))),
            ),
            (
                "Summary",
                self.summary
                    .clone()
                    .or_else(|| meta.description.clone())
                    .or_else(|| Some(config.app_name.clone())),
            ),
            ("Group", self.group.clone()),
            ("Vendor", self.vendor.clone()),
            (
                "Packager",
                match (&self.packager, &self.packager_email) {
                    (Some(p), Some(e)) => Some(format!("{} <{}>", p, e)),
                    (p, None) => p.clone(),
                    (None, Some(e)) => Some(format!(" <{}>", e)),
                },
            ),
            ("License", self.license.clone()),
            ("URL", self.url.clone()),
        ];
        if let Some(requires) = self.requires.as_ref().filter(|v| !v.is_empty()) {
            preamble.push(("Requires", Some(requires.join(", "))));
        }
        if let Some(build_requires) = self.build_requires.as_ref().filter(|v| !v.is_empty()) {
            preamble.push(("BuildRequires", Some(build_requires.join(", "))));
        }
        preamble.push(("BuildArch", Some(self.build_arch())));

        let preamble_str = preamble
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| format!("{}: {}", k, v)))
            .collect::<Vec<_>>()
            .join("\n");

        // Body
        let app_name = &config.app_name;
        let binary_name = &config.app_binary_name;
        let install_script = [
            "mkdir -p %{buildroot}%{_bindir}".to_string(),
            "mkdir -p %{buildroot}%{_datadir}/%{name}".to_string(),
            "mkdir -p %{buildroot}%{_datadir}/applications".to_string(),
            "mkdir -p %{buildroot}%{_datadir}/metainfo".to_string(),
            "mkdir -p %{buildroot}%{_datadir}/pixmaps".to_string(),
            format!("cp -r {}/* %{{buildroot}}%{{_datadir}}/%{{name}}", app_name),
            format!(
                "ln -s %{{_datadir}}/%{{name}}/{} %{{buildroot}}%{{_bindir}}/%{{name}}",
                binary_name
            ),
            format!(
                "cp -r {}.desktop %{{buildroot}}%{{_datadir}}/applications/%{{name}}.desktop",
                binary_name
            ),
            format!(
                "cp -r {}.png %{{buildroot}}%{{_datadir}}/pixmaps/%{{name}}.png",
                binary_name
            ),
            format!(
                "cp -r {}*.xml %{{buildroot}}%{{_datadir}}/metainfo/%{{name}}.appdata.xml || :",
                binary_name
            ),
        ]
        .join("\n");

        // %post always refreshes the MIME database, then runs any custom
        // scripts (mirrors Dart's postScripts/postunScripts getters).
        let mut post_scripts =
            vec!["update-mime-database %{_datadir}/mime &> /dev/null || :".to_string()];
        post_scripts.extend(self.postinstall_scripts.clone().unwrap_or_default());

        let mut postun_scripts =
            vec!["update-mime-database %{_datadir}/mime &> /dev/null || :".to_string()];
        if let Some(postun) = &self.postun {
            postun_scripts.push(postun.clone());
        }
        postun_scripts.extend(self.postuninstall_scripts.clone().unwrap_or_default());

        let files_section = [
            "%{_bindir}/%{name}",
            "%{_datadir}/%{name}",
            "%{_datadir}/applications/%{name}.desktop",
            "%{_datadir}/metainfo",
        ]
        .join("\n");

        let body = [
            format!("%description\n{}\n", description),
            format!("%install\n{}\n", install_script),
            format!("%post\n{}\n", post_scripts.join("\n")),
            format!("%postun\n{}\n", postun_scripts.join("\n")),
            format!("%files\n{}\n", files_section),
        ]
        .join("\n");

        let inline_body = [
            "%defattr(-,root,root)\n",
            "%attr(4755, root, root) %{_datadir}/pixmaps/%{name}.png\n",
        ]
        .join("\n");

        let macros = self
            .spec_macros
            .as_ref()
            .filter(|v| !v.is_empty())
            .map(|v| format!("{}\n\n", v.join("\n")))
            .unwrap_or_default();

        format!("{}{}\n\n{}\n\n{}", macros, preamble_str, body, inline_body)
    }

    /// Renders the desktop entry, mirroring Dart's `toJson()['DESKTOP']`.
    fn desktop_file(&self, config: &PackageConfig) -> String {
        let rpm_name = self.rpm_name(config);
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
            ("Icon", Some(rpm_name.clone())),
            ("Exec", Some(format!("{} %U", rpm_name))),
            ("Actions", desktop_list(&self.actions)),
            ("MimeType", desktop_list(&self.supported_mime_type)),
            ("Categories", desktop_list(&self.categories)),
            ("Keywords", desktop_list(&self.keywords)),
            (
                "StartupNotify",
                Some(self.startup_notify.unwrap_or(true).to_string()),
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

/// Runs `patchelf --print-rpath` / `--set-rpath` on every `lib/*.so` in the
/// bundle, replacing absolute RPATH entries with `$ORIGIN` (mirrors the Dart
/// maker's RPATH fix for https://github.com/flutter/flutter/issues/65400).
fn sanitize_bundle_rpaths(build_root: &Path) -> Result<(), PackageError> {
    let lib_dir = build_root.join("lib");
    let Ok(entries) = std::fs::read_dir(&lib_dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|e| e != "so") {
            continue;
        }
        let out = Command::new("patchelf")
            .args(["--print-rpath", &path.display().to_string()])
            .output()
            .map_err(|e| PackageError::MissingTool(format!("patchelf: {}", e)))?;
        if !out.status.success() {
            continue;
        }
        let rpath = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let sanitized = sanitize_rpm_rpath(&rpath);
        if sanitized != rpath {
            run(Command::new("patchelf").args([
                "--set-rpath",
                &sanitized,
                &path.display().to_string(),
            ]))?;
        }
    }
    Ok(())
}

impl AppPackager for LinuxRpmPackager {
    fn name(&self) -> &str {
        "rpm"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        "rpm"
    }

    #[cfg(not(target_os = "linux"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = RpmMakeConfig::load()?;
        let pkg_dir = config.packaging_dir();
        let binary_name = &config.app_binary_name;
        let app_name = &config.app_name;

        // Create rpmbuild tree: BUILD BUILDROOT RPMS SOURCES SPECS SRPMS
        let rpmbuild_dir = pkg_dir.join("rpmbuild");
        for sub in &["BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"] {
            std::fs::create_dir_all(rpmbuild_dir.join(sub))?;
        }

        // Copy app files into BUILD/{app_name}/
        let build_dir = rpmbuild_dir.join("BUILD");
        let build_root = build_dir.join(app_name);
        std::fs::create_dir_all(&build_root)?;
        run(Command::new("cp").args([
            "-fr",
            &format!("{}/.", config.build_output_dir.display()),
            &build_root.display().to_string(),
        ]))?;

        // Fix lib_*_plugin.so RPATHs pointing at the build directory
        sanitize_bundle_rpaths(&build_root)?;

        // Copy the configured icon into BUILD/<binary><ext>
        if let Some(icon) = &make_config.icon {
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
            std::fs::copy(icon_path, build_dir.join(format!("{}{}", binary_name, ext)))?;
        }

        // Copy the configured metainfo into BUILD/<binary><ext2>
        if let Some(metainfo) = &make_config.metainfo {
            let metainfo_path = Path::new(metainfo);
            if !metainfo_path.exists() {
                return Err(PackageError::NotFound(format!(
                    "Metainfo {} path doesn't exist",
                    metainfo
                )));
            }
            let file_name = metainfo_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            let ext = {
                let parts: Vec<&str> = file_name.split('.').collect();
                match parts.len() {
                    0 | 1 => String::new(),
                    2 => format!(".{}", parts[1]),
                    n => format!(".{}.{}", parts[n - 2], parts[n - 1]),
                }
            };
            std::fs::copy(
                metainfo_path,
                build_dir.join(format!("{}{}", binary_name, ext)),
            )?;
        }

        // Write BUILD/{binary_name}.desktop and SPECS/{binary_name}.spec
        std::fs::write(
            build_dir.join(format!("{}.desktop", binary_name)),
            make_config.desktop_file(config),
        )?;
        let spec_path = rpmbuild_dir
            .join("SPECS")
            .join(format!("{}.spec", binary_name));
        std::fs::write(&spec_path, make_config.spec_file(config))?;

        // QA_RPATHS = 0x0001 | 0x0010 tolerates $ORIGIN-style RPATHs
        run(Command::new("rpmbuild")
            .args([
                "--define",
                &format!("_topdir {}", rpmbuild_dir.display()),
                "-bb",
                &spec_path.display().to_string(),
            ])
            .env("QA_RPATHS", (0x0001 | 0x0010).to_string()))?;

        // Find the produced RPM and copy it to the output file
        let rpm_dir = rpmbuild_dir.join("RPMS").join(make_config.build_arch());
        let output_file = config.output_file();
        let entries: Vec<_> = std::fs::read_dir(&rpm_dir)
            .map_err(|_| PackageError::NotFound(rpm_dir.display().to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .collect();
        let first_rpm = entries
            .first()
            .ok_or_else(|| PackageError::General("rpmbuild produced no output".into()))?;
        std::fs::copy(first_rpm.path(), &output_file)?;

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
            package_format: "rpm".into(),
            is_installer: false,
            build_output_dir: PathBuf::new(),
            build_output_files: vec![],
            output_dir: PathBuf::new(),
        }
    }

    #[test]
    fn sanitize_rpath_replaces_absolute_and_dedupes() {
        assert_eq!(
            sanitize_rpm_rpath("/home/user/build/lib:/opt/x:$ORIGIN"),
            "$ORIGIN"
        );
        assert_eq!(
            sanitize_rpm_rpath("$ORIGIN/../lib:/home/user/build"),
            "$ORIGIN/../lib:$ORIGIN"
        );
        assert_eq!(sanitize_rpm_rpath(""), "");
    }

    #[test]
    fn spec_file_contains_configured_fields() {
        let mc: RpmMakeConfig = serde_yaml::from_str(
            r#"
display_name: Hola Amigos
package_name: hola-amigos
summary: An awesome app
group: Applications/Multimedia
vendor: ACME
packager: Gamer Boy 69
packager_email: rickastley@gmail.lol
license: MIT
url: https://example.com
requires:
  - libkeybinder
postinstall_scripts:
  - echo Installed
postun: echo Uninstalling
spec_macros:
  - "%define _build_id_links none"
"#,
        )
        .unwrap();
        let spec = mc.spec_file(&test_config());
        assert!(spec.starts_with("%define _build_id_links none"));
        assert!(spec.contains("Name: hola-amigos"));
        assert!(spec.contains("Version: 1.2.3+4"));
        assert!(spec.contains("Release: 4%{?dist}"));
        assert!(spec.contains("Summary: An awesome app"));
        assert!(spec.contains("Packager: Gamer Boy 69 <rickastley@gmail.lol>"));
        assert!(spec.contains("License: MIT"));
        assert!(spec.contains("Requires: libkeybinder"));
        assert!(spec.contains("%post\nupdate-mime-database %{_datadir}/mime &> /dev/null || :\necho Installed"));
        assert!(spec.contains("echo Uninstalling"));
        assert!(spec.contains("%attr(4755, root, root)"));
    }

    #[test]
    fn spec_defaults_without_make_config() {
        let mc = RpmMakeConfig::default();
        let spec = mc.spec_file(&test_config());
        assert!(spec.contains("Name: hola_amigos"));
        assert!(spec.contains("Release: 4%{?dist}"));
        assert!(spec.contains("%description"));
        assert!(spec.contains("%files"));
    }

    #[test]
    fn desktop_uses_rpm_name_for_exec() {
        let mc: RpmMakeConfig =
            serde_yaml::from_str("display_name: Hola\npackage_name: hola-amigos\n").unwrap();
        let desktop = mc.desktop_file(&test_config());
        assert!(desktop.contains("Name=Hola"));
        assert!(desktop.contains("Icon=hola-amigos"));
        assert!(desktop.contains("Exec=hola-amigos %U"));
    }
}
