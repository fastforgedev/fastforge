use std::path::Path;
use std::process::Command;

use fastforge_core::{
    AppPackager, PackageConfig, PackageError, PackageResult, Platform, Variables,
};
use serde::Deserialize;

use super::common::{
    FormatVariables, RawPackaging, desktop_categories, desktop_list, load_make_config,
    machine_architecture, render_desktop_entry, var,
};
use super::staging::{self, Contents, Layout};

/// Builds an RPM package using `rpmbuild`, mirroring Dart's `AppPackageMakerRPM`.
///
/// The package root is staged by [`staging::stage`] (bundle in
/// `/opt/<binary>`, `/usr/bin/<binary>` link, desktop entry, icons,
/// metainfo, `files/` overlays) at `${PACKAGING_DIRECTORY}`, with its
/// `%files` list at `${RPM_FILE_LIST}`, so a spec installs it with
/// `cp -a ${PACKAGING_DIRECTORY}/. %{buildroot}/` and
/// `%files -f ${RPM_FILE_LIST}`. A raw `.spec` in
/// `.fastforge/packaging/linux/rpm/` (its file name rendered too) is rendered
/// with fastforge's variables, which are also set in rpmbuild's environment
/// (`%{getenv:APP_VERSION}`), and wins over the spec generated from
/// `linux/packaging/rpm/make_config.yaml` (same schema as Dart's
/// `MakeRPMConfig`) or defaults.
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
    machine_architecture().to_string()
}

/// RPM `Release` from the app version, mirroring Dart's
/// `appVersion.build.first`: the first dot-separated part of the build
/// number (`1.2.3+4.5` → `4`), numeric parts normalized like pub_semver
/// (`+04` → `4`), `1` when there is no build number.
fn rpm_release(app_version: &str) -> String {
    let Some((_, build)) = app_version.split_once('+') else {
        return "1".to_string();
    };
    let first = build.split('.').next().unwrap_or_default();
    if first.is_empty() {
        return "1".to_string();
    }
    match first.parse::<u64>() {
        Ok(n) => n.to_string(),
        Err(_) => first.to_string(),
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

/// RPM `Version`: the build name, with `-` (not allowed) turned into `~`, so
/// a pre-release (`1.2.3-beta.1` -> `1.2.3~beta.1`) sorts before the release.
fn rpm_version(app_version: &str) -> String {
    app_version
        .split('+')
        .next()
        .unwrap_or(app_version)
        .replace('-', "~")
}

/// `RPM_PRIVATE_LIBS`: the bundle's shared libraries (`lib/*.so*`) as a
/// regular expression alternation of their `<name>.so` stems, escaped for a
/// spec (see [`regex_escape`]), e.g. `libapp\\.so|libflutter_linux_gtk\\.so`,
/// for `%__requires_exclude`.
fn private_libs(lib_dir: &Path) -> String {
    let Ok(entries) = std::fs::read_dir(lib_dir) else {
        return String::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let end = name.find(".so")? + ".so".len();
            Some(regex_escape(&name[..end]))
        })
        .collect();
    names.sort();
    names.dedup();
    names.join("|")
}

/// Escapes a value for the regular expressions of rpm's dependency filters,
/// as written in a spec: rpm consumes one level of backslashes when it
/// expands `%global`, so `.` is written `\\.` to reach the regex as `\.`.
fn regex_escape(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '\\' => {
                format!("\\\\{}", c)
            }
            c => c.to_string(),
        })
        .collect()
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

    /// Renders the default `.spec`: it copies the staged root
    /// (`PACKAGING_DIRECTORY`) and lists it with `%files -f RPM_FILE_LIST`.
    /// The bundle's private libraries are kept out of the automatic
    /// Provides/Requires, no debuginfo package or `.build-id` links are made
    /// (they would clash between apps shipping the same Flutter engine), and
    /// project metadata fills what `make_config.yaml` leaves out (`Summary`
    /// and `License` are mandatory).
    fn spec_file(
        &self,
        config: &PackageConfig,
        variables: &Variables,
        install_dir: &str,
    ) -> String {
        let root = var(variables, "PACKAGING_DIRECTORY").unwrap_or_default();
        let file_list = var(variables, "RPM_FILE_LIST").unwrap_or_default();
        let fallback_description = || {
            var(variables, "APP_DESCRIPTION")
                .or_else(|| var(variables, "APP_DISPLAY_NAME"))
                .unwrap_or_else(|| config.app_name.clone())
        };
        let bundle = regex_escape(install_dir);

        let mut macros = vec![
            "%global debug_package %{nil}".to_string(),
            "%global _build_id_links none".to_string(),
            format!("%global __provides_exclude_from ^{}/.*$", bundle),
        ];
        // Only the bundle's own libraries are dropped from the automatic
        // Requires; system libraries (GTK, glibc, ...) stay required.
        if let Some(private) = var(variables, "RPM_PRIVATE_LIBS") {
            macros.push(format!("%global __requires_exclude ^({})", private));
        }
        macros.extend(self.spec_macros.clone().unwrap_or_default());

        let packager = match (&self.packager, &self.packager_email) {
            (Some(p), Some(e)) => Some(format!("{} <{}>", p, e)),
            (Some(p), None) => Some(p.clone()),
            (None, Some(e)) => Some(format!("<{}>", e)),
            (None, None) => var(variables, "APP_MAINTAINER"),
        };
        let list =
            |v: &Option<Vec<String>>| v.as_ref().filter(|v| !v.is_empty()).map(|v| v.join(", "));
        let preamble: Vec<(&str, Option<String>)> = vec![
            (
                "Name",
                Some(var(variables, "PACKAGE_NAME").unwrap_or_else(|| self.rpm_name(config))),
            ),
            (
                "Version",
                Some(
                    var(variables, "PACKAGE_VERSION")
                        .unwrap_or_else(|| rpm_version(&config.app_version)),
                ),
            ),
            (
                "Release",
                Some(format!("{}%{{?dist}}", rpm_release(&config.app_version))),
            ),
            (
                "Summary",
                Some(self.summary.clone().unwrap_or_else(fallback_description)),
            ),
            (
                "License",
                Some(
                    self.license
                        .clone()
                        .or_else(|| var(variables, "APP_LICENSE"))
                        .unwrap_or_else(|| "LicenseRef-Unknown".to_string()),
                ),
            ),
            (
                "URL",
                self.url.clone().or_else(|| var(variables, "APP_HOMEPAGE")),
            ),
            ("Group", self.group.clone()),
            ("Vendor", self.vendor.clone()),
            ("Packager", packager),
            ("Requires", list(&self.requires)),
            ("BuildRequires", list(&self.build_requires)),
            ("BuildArch", Some(self.build_arch())),
        ];
        let preamble = preamble
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| format!("{}: {}", k, v)))
            .collect::<Vec<_>>()
            .join("\n");

        let mut body = vec![
            format!(
                "%description\n{}\n",
                self.description
                    .clone()
                    .unwrap_or_else(fallback_description)
            ),
            format!("%install\ncp -a {}/. %{{buildroot}}/\n", root),
        ];
        if let Some(scripts) = self.postinstall_scripts.as_ref().filter(|s| !s.is_empty()) {
            body.push(format!("%post\n{}\n", scripts.join("\n")));
        }
        let mut postun: Vec<String> = self.postun.iter().cloned().collect();
        postun.extend(self.postuninstall_scripts.clone().unwrap_or_default());
        if !postun.is_empty() {
            body.push(format!("%postun\n{}\n", postun.join("\n")));
        }
        body.push(format!("%files -f {}\n", file_list));

        format!(
            "{}\n\n{}\n\n{}",
            macros.join("\n"),
            preamble,
            body.join("\n")
        )
    }

    /// Renders the default desktop entry.
    fn desktop_file(&self, config: &PackageConfig, icon: &str) -> String {
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
            ("Exec", Some(format!("{} %U", config.app_binary_name))),
            ("Actions", desktop_list(&self.actions)),
            ("MimeType", desktop_list(&self.supported_mime_type)),
            ("Categories", desktop_categories(&self.categories)),
            ("Keywords", desktop_list(&self.keywords)),
            // Only written when `startup_notify` is configured (like Dart).
            ("StartupNotify", self.startup_notify.map(|b| b.to_string())),
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
        // rpmbuild needs absolute paths (Dart uses
        // `packagingDirectory.absolute.path`).
        let pkg_dir = std::path::absolute(config.packaging_dir())?;
        let output_file = config.output_file();
        let rpmbuild_dir = pkg_dir.join("rpmbuild");
        for sub in &["BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"] {
            std::fs::create_dir_all(rpmbuild_dir.join(sub))?;
        }
        let root = pkg_dir.join("root");
        let file_list = pkg_dir.join("files.list");
        let binary_name = &config.app_binary_name;
        let layout = Layout::system(binary_name);
        let raw = RawPackaging::load(
            config,
            "rpm",
            FormatVariables {
                package_name: make_config.package_name.clone(),
                default_package_name: config.app_name.clone(),
                package_version: rpm_version(&config.app_version),
                package_arch: make_config.build_arch(),
                install_dir: Some(layout.install_dir()),
                display_name: make_config.display_name.clone(),
                packaging_dir: &root,
                output_file: &output_file,
                extra: vec![
                    ("RPM_RELEASE", rpm_release(&config.app_version)),
                    ("RPM_FILE_LIST", file_list.display().to_string()),
                    (
                        "RPM_PRIVATE_LIBS",
                        private_libs(&config.build_output_dir.join("lib")),
                    ),
                ],
            },
        )?;

        staging::stage(
            &raw,
            config,
            &root,
            &layout,
            Contents {
                icon: raw.icon(make_config.icon.as_ref()),
                metainfo: make_config.metainfo.clone(),
            },
            || make_config.desktop_file(config, raw.app_id()),
        )?;
        // Fix lib_*_plugin.so RPATHs pointing at the build directory
        sanitize_bundle_rpaths(&root.join(&layout.bundle_dir))?;
        std::fs::write(&file_list, staging::rpm_file_list(&root)?)?;

        // The spec (the format directory's only `*.spec`, whatever its name)
        // is written as `SPECS/<package>.spec`, as rpm conventionally names it.
        let raw_spec = raw.file_with_extension("spec")?;
        let spec_path = rpmbuild_dir
            .join("SPECS")
            .join(format!("{}.spec", raw.package_name()));
        raw.write_or_generate(raw_spec, &spec_path, || {
            make_config.spec_file(config, raw.variables(), &layout.install_dir())
        })?;

        // QA_RPATHS = 0x0001 | 0x0010 tolerates $ORIGIN-style RPATHs
        let mut cmd = Command::new("rpmbuild");
        cmd.args([
            "--define",
            &format!("_topdir {}", rpmbuild_dir.display()),
            "-bb",
            &spec_path.display().to_string(),
        ]);
        raw.apply_env(&mut cmd);
        cmd.env("QA_RPATHS", (0x0001 | 0x0010).to_string());
        run(&mut cmd)?;

        // Copy the produced RPM to the output file. A raw spec may set its
        // own BuildArch, so look in every RPMS/<arch>/ directory.
        let produced = find_rpm(&rpmbuild_dir.join("RPMS"))
            .ok_or_else(|| PackageError::General("rpmbuild produced no output".into()))?;
        std::fs::copy(produced, &output_file)?;

        std::fs::remove_dir_all(&pkg_dir).ok();
        config.resolve_result(output_file)
    }
}

/// The first `.rpm` file (by path) under `dir`.
fn find_rpm(dir: &Path) -> Option<std::path::PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if let Some(found) = find_rpm(&path) {
                return Some(found);
            }
        } else if path.extension().is_some_and(|e| e == "rpm") {
            return Some(path);
        }
    }
    None
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
            environment: Default::default(),
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

    fn variables(entries: &[(&str, &str)]) -> Variables {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn staged() -> Variables {
        variables(&[
            ("PACKAGING_DIRECTORY", "/tmp/pkg/root"),
            ("RPM_FILE_LIST", "/tmp/pkg/files.list"),
            ("RPM_PRIVATE_LIBS", r"libapp\\.so|libflutter_linux_gtk\\.so"),
        ])
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
        let spec = mc.spec_file(&test_config(), &staged(), "/opt/hola_amigos");
        assert!(spec.starts_with(
            "%global debug_package %{nil}\n\
             %global _build_id_links none\n\
             %global __provides_exclude_from ^/opt/hola_amigos/.*$\n\
             %global __requires_exclude ^(libapp\\\\.so|libflutter_linux_gtk\\\\.so)\n\
             %define _build_id_links none\n"
        ));
        assert!(spec.contains("Name: hola-amigos\n"));
        assert!(spec.contains("Version: 1.2.3\n"));
        assert!(spec.contains("Release: 4%{?dist}\n"));
        assert!(spec.contains("Summary: An awesome app\n"));
        assert!(spec.contains("Packager: Gamer Boy 69 <rickastley@gmail.lol>\n"));
        assert!(spec.contains("License: MIT\n"));
        assert!(spec.contains("Requires: libkeybinder\n"));
        assert!(spec.contains("%install\ncp -a /tmp/pkg/root/. %{buildroot}/\n"));
        assert!(spec.contains("%post\necho Installed\n"));
        assert!(spec.contains("%postun\necho Uninstalling\n"));
        assert!(spec.ends_with("%files -f /tmp/pkg/files.list\n"));
        // No setuid icon, no system directory ownership.
        assert!(!spec.contains("4755"));
        assert!(!spec.contains("%{_datadir}/metainfo"));
    }

    #[test]
    fn spec_defaults_without_make_config() {
        let mc = RpmMakeConfig::default();
        let spec = mc.spec_file(&test_config(), &staged(), "/opt/hola_amigos");
        assert!(spec.contains("Name: hola_amigos\n"));
        assert!(spec.contains("Release: 4%{?dist}\n"));
        // Summary and License are mandatory: defaults stand in.
        assert!(spec.contains("Summary: hola_amigos\n"));
        assert!(spec.contains("License: LicenseRef-Unknown\n"));
        assert!(spec.contains("%description\nhola_amigos\n"));
        assert!(!spec.contains("Requires"));
        assert!(!spec.contains("%post"));

        let spec = mc.spec_file(
            &test_config(),
            &variables(&[
                ("APP_DESCRIPTION", "A demo"),
                ("APP_LICENSE", "Apache-2.0"),
                ("APP_MAINTAINER", "Jane <jane@example.com>"),
                ("APP_HOMEPAGE", "https://example.com"),
            ]),
            "/opt/hola_amigos",
        );
        assert!(spec.contains("Summary: A demo\n"));
        assert!(spec.contains("License: Apache-2.0\n"));
        assert!(spec.contains("Packager: Jane <jane@example.com>\n"));
        assert!(spec.contains("URL: https://example.com\n"));
    }

    #[test]
    fn versions_and_paths_follow_rpm_rules() {
        assert_eq!(rpm_version("1.2.3+4"), "1.2.3");
        assert_eq!(rpm_version("1.2.3-beta.1+4"), "1.2.3~beta.1");
        assert_eq!(regex_escape("/opt/my.app+x"), r"/opt/my\\.app\\+x");
    }

    #[test]
    fn private_libs_are_the_bundled_shared_objects() {
        let tmp = tempfile::tempdir().unwrap();
        for name in ["libapp.so", "libfoo_plugin.so", "libbar.so.1.2", "README"] {
            std::fs::write(tmp.path().join(name), "").unwrap();
        }
        assert_eq!(
            private_libs(tmp.path()),
            r"libapp\\.so|libbar\\.so|libfoo_plugin\\.so"
        );
        assert_eq!(private_libs(&tmp.path().join("missing")), "");
    }

    #[test]
    fn release_uses_first_build_number_part() {
        assert_eq!(rpm_release("1.2.3+4.5"), "4");
        assert_eq!(rpm_release("1.2.3+04"), "4");
        assert_eq!(rpm_release("1.2.3+beta.2"), "beta");
        assert_eq!(rpm_release("1.2.3"), "1");
    }

    #[test]
    fn desktop_startup_notify_only_when_configured() {
        let desktop = RpmMakeConfig::default().desktop_file(&test_config(), "hola_amigos");
        assert!(!desktop.contains("StartupNotify"));
        let mc: RpmMakeConfig = serde_yaml::from_str("startup_notify: true\n").unwrap();
        assert!(
            mc.desktop_file(&test_config(), "hola_amigos")
                .contains("StartupNotify=true")
        );
    }

    #[test]
    fn desktop_execs_the_binary() {
        let mc: RpmMakeConfig =
            serde_yaml::from_str("display_name: Hola\npackage_name: hola-amigos\n").unwrap();
        let desktop = mc.desktop_file(&test_config(), "dev.example.hola");
        assert!(desktop.contains("Name=Hola"));
        assert!(desktop.contains("Icon=dev.example.hola"));
        assert!(desktop.contains("Exec=hola_amigos %U"));
    }

    #[test]
    fn find_rpm_looks_in_every_arch_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let rpms = tmp.path().join("RPMS");
        assert_eq!(find_rpm(&rpms), None);
        std::fs::create_dir_all(rpms.join("noarch")).unwrap();
        std::fs::write(rpms.join("noarch/readme.txt"), "").unwrap();
        assert_eq!(find_rpm(&rpms), None);
        std::fs::write(rpms.join("noarch/demo-1.0-1.noarch.rpm"), "").unwrap();
        assert_eq!(
            find_rpm(&rpms),
            Some(rpms.join("noarch/demo-1.0-1.noarch.rpm"))
        );
    }
}
