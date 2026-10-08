use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

use super::common::{
    FormatVariables, RawPackaging, load_make_config, machine_architecture, make_executable,
};
use super::staging::{self, Contents, Layout};

/// Builds a Linux AppImage using `appimagetool`, mirroring Dart's
/// `AppPackageMakerAppImage`.
///
/// The AppDir is staged by [`staging::stage`] (bundle, desktop entry and
/// icon at the root, icons and metainfo under `usr/share`, `files/`
/// overlays). A raw `AppRun` in `.fastforge/packaging/linux/appimage/` is
/// rendered with fastforge's variables and wins over the one generated from
/// `linux/packaging/appimage/make_config.yaml` (same schema as Dart's
/// `MakeAppImageConfig`) or defaults.
/// A `.desktop` template and a `files/` overlay in
/// `.fastforge/packaging/linux/shared/` are shared by every format.
///
/// Requires `appimagetool` (plus `ldd` and `locate` for dependency bundling)
/// to be on `$PATH`.
pub struct LinuxAppImagePackager;

/// A desktop action entry (`[Desktop Action <label>]`), mirroring Dart's
/// `AppImageAction`.
#[derive(Debug, Clone, Deserialize)]
pub struct AppImageAction {
    pub label: String,
    pub name: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}

/// Schema of `linux/packaging/appimage/make_config.yaml`, mirroring Dart's
/// `MakeAppImageConfig.fromJson`.
#[derive(Debug, Default, Deserialize)]
pub struct AppImageMakeConfig {
    pub display_name: Option<String>,
    pub icon: Option<String>,
    pub metainfo: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub actions: Vec<AppImageAction>,
    pub startup_notify: Option<bool>,
    pub generic_name: Option<String>,
    pub supported_mime_type: Option<Vec<String>>,
}

impl AppImageMakeConfig {
    fn load() -> Result<Self, PackageError> {
        Ok(
            load_make_config(Path::new("linux/packaging/appimage/make_config.yaml"))?
                .unwrap_or_default(),
        )
    }

    /// Renders the desktop file, mirroring Dart's `desktopFileContent`
    /// (including `[Desktop Action]` sections).
    fn desktop_file(&self, config: &PackageConfig, icon: &str) -> String {
        let app_name = &config.app_name;
        // The executable is `BINARY_NAME` from `linux/CMakeLists.txt`, which
        // need not match the pubspec name that the AppDir files are named after.
        let binary_name = &config.app_binary_name;
        let mut fields: Vec<(&str, String)> = vec![
            (
                "Name",
                self.display_name
                    .clone()
                    .unwrap_or_else(|| app_name.clone()),
            ),
            (
                "GenericName",
                self.generic_name
                    .clone()
                    .unwrap_or_else(|| "A Flutter Application".to_string()),
            ),
            ("Exec", format!("{} %U", binary_name)),
            ("Icon", icon.to_string()),
            ("Type", "Application".to_string()),
            (
                "StartupNotify",
                self.startup_notify.unwrap_or(false).to_string(),
            ),
        ];
        if let Some(mime) = self.supported_mime_type.as_ref().filter(|v| !v.is_empty()) {
            fields.push(("MimeType", format!("{};", mime.join(";"))));
        }
        // The spec expects a main category, and appimagetool requires one.
        let categories = if self.categories.is_empty() {
            "Utility;".to_string()
        } else {
            self.categories.join(";")
        };
        fields.push(("Categories", categories));
        if !self.keywords.is_empty() {
            fields.push(("Keywords", self.keywords.join(";")));
        }
        if !self.actions.is_empty() {
            fields.push((
                "Actions",
                self.actions
                    .iter()
                    .map(|a| a.label.clone())
                    .collect::<Vec<_>>()
                    .join(";"),
            ));
        }

        let entry = fields
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        let actions = self
            .actions
            .iter()
            .map(|action| {
                let exec = [binary_name.clone(), action.arguments.join(" ")]
                    .join(" ")
                    .trim()
                    .to_string();
                format!(
                    "[Desktop Action {}]\nName={}\nExec={}",
                    action.label, action.name, exec,
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!("[Desktop Entry]\n{}\n\n{}", entry, actions)
    }

    /// Renders the default `AppRun`: it puts the bundled libraries first and
    /// starts the binary with the given arguments, without changing the
    /// working directory (so relative paths keep working).
    fn app_run(&self, config: &PackageConfig) -> String {
        format!(
            "#!/bin/sh\n\
             HERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\n\
             export LD_LIBRARY_PATH=\"$HERE/usr/lib:$HERE/lib${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}\"\n\
             exec \"$HERE/{}\" \"$@\"\n",
            config.app_binary_name
        )
    }
}

fn run(cmd: &mut Command) -> Result<(), PackageError> {
    let out = cmd.output().map_err(|e| {
        PackageError::MissingTool(format!("{}: {}", cmd.get_program().to_string_lossy(), e))
    })?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: cmd.get_program().to_string_lossy().into(),
            stderr: failure_output(&out.stdout, &out.stderr),
        });
    }
    Ok(())
}

/// stderr followed by stdout: appimagetool prints the reason for some
/// failures, such as `desktop-file-validate` errors, only on stdout.
fn failure_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stdout = String::from_utf8_lossy(stdout);
    match (stderr.trim(), stdout.trim()) {
        (err, "") => err.to_string(),
        ("", out) => out.to_string(),
        (err, out) => format!("{}\n{}", err, out),
    }
}

fn run_stdout(cmd: &mut Command) -> Result<String, PackageError> {
    let out = cmd.output().map_err(|e| {
        PackageError::MissingTool(format!("{}: {}", cmd.get_program().to_string_lossy(), e))
    })?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: cmd.get_program().to_string_lossy().into(),
            stderr: String::from_utf8_lossy(&out.stderr).into(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Parses `ldd` output into resolved shared-object paths, mirroring Dart's
/// `_getSharedDependencies`.
fn parse_ldd_output(output: &str) -> BTreeSet<String> {
    output
        .lines()
        .filter(|line| line.contains("=>") && line.trim().starts_with("lib"))
        .filter_map(|line| {
            line.split(" => ")
                .nth(1)
                .and_then(|rest| rest.trim().split(' ').next())
                .map(|s| s.trim().to_string())
        })
        .collect()
}

fn shared_dependencies(so_path: &Path) -> Result<BTreeSet<String>, PackageError> {
    let output = run_stdout(Command::new("ldd").args(["-d", &so_path.display().to_string()]))?;
    Ok(parse_ldd_output(&output))
}

impl AppPackager for LinuxAppImagePackager {
    fn name(&self) -> &str {
        "appimage"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        "AppImage"
    }

    #[cfg(not(target_os = "linux"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = AppImageMakeConfig::load()?;

        // The artifact uses the `.AppImage` extension (mirrors Dart, which
        // overrides packageFormat to `AppImage` for the output file).
        let mut effective = config.clone();
        effective.package_format = "AppImage".to_string();
        let output_file = effective.output_file();

        let pkg_dir = config.packaging_dir();
        let app_name = &config.app_name;

        let app_dir = pkg_dir.join(format!("{}.AppDir", app_name));
        std::fs::create_dir_all(&app_dir)?;
        let arch = machine_architecture();
        let raw = RawPackaging::load(
            &effective,
            "appimage",
            FormatVariables {
                package_name: None,
                default_package_name: app_name.clone(),
                package_version: config.app_version.clone(),
                package_arch: arch.to_string(),
                install_dir: None,
                display_name: make_config.display_name.clone(),
                packaging_dir: &app_dir,
                output_file: &output_file,
                extra: vec![],
            },
        )?;

        staging::stage(
            &raw,
            config,
            &app_dir,
            &Layout::app_dir(),
            Contents {
                icon: raw.icon(make_config.icon.as_ref()),
                metainfo: make_config.metainfo.clone(),
            },
            || make_config.desktop_file(config, raw.app_id()),
        )?;
        let app_run_path = app_dir.join("AppRun");
        raw.write_or_generate(raw.file(&["AppRun"]), &app_run_path, || {
            make_config.app_run(config)
        })?;
        make_executable(&app_run_path)?;

        // Bundle shared-object dependencies of plugin libraries into usr/lib
        // (mirrors Dart: deps of each lib/*.so, minus the flutter GTK deps).
        let usr_lib = app_dir.join("usr/lib");
        std::fs::create_dir_all(&usr_lib)?;

        let default_shared_objects = ["libapp.so", "libflutter_linux_gtk.so", "libgtk-3.so.0"];
        let lib_dir = app_dir.join("lib");
        let gtk_so = lib_dir.join("libflutter_linux_gtk.so");
        if gtk_so.exists() {
            let gtk_deps = shared_dependencies(&gtk_so)?;
            if let Ok(entries) = std::fs::read_dir(&lib_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let base = path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if default_shared_objects.contains(&base.as_str()) {
                        continue;
                    }
                    let mut deps = shared_dependencies(&path)?;
                    deps = deps.difference(&gtk_deps).cloned().collect();
                    deps.retain(|lib| !lib.contains("libflutter_linux_gtk.so"));
                    // Libraries the `files/` overlay provides win.
                    deps.retain(|lib| {
                        Path::new(lib)
                            .file_name()
                            .is_none_or(|name| !usr_lib.join(name).exists())
                    });
                    if deps.is_empty() {
                        continue;
                    }
                    let mut args: Vec<String> = deps.into_iter().collect();
                    args.push(usr_lib.display().to_string());
                    run(Command::new("cp").args(&args))?;
                }
            }
        }

        // Copy explicitly included shared objects (resolved via `locate`)
        for so in &make_config.include {
            let output = run_stdout(Command::new("locate").arg(so))?;
            let found = output
                .lines()
                .map(str::trim)
                .find(|p| !p.is_empty() && !p.contains("/Trash"))
                .ok_or_else(|| {
                    PackageError::NotFound(format!("Can't find specified shared object {}", so))
                })?;
            let src = Path::new(found);
            let base = src
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            std::fs::copy(src, usr_lib.join(base))?;
        }

        // Build the AppImage
        let mut cmd = Command::new("appimagetool");
        cmd.args([
            "--no-appstream",
            &app_dir.display().to_string(),
            &output_file.display().to_string(),
        ]);
        raw.apply_env(&mut cmd);
        run(&mut cmd)?;

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
            package_format: "appimage".into(),
            is_installer: false,
            build_output_dir: PathBuf::new(),
            build_output_files: vec![],
            output_dir: PathBuf::new(),
            environment: Default::default(),
        }
    }

    #[test]
    fn desktop_file_with_actions() {
        let mc: AppImageMakeConfig = serde_yaml::from_str(
            r#"
display_name: Hola Amigos
icon: assets/logo.png
categories:
  - Music
  - Media
keywords:
  - Hello
supported_mime_type:
  - audio/mpeg
actions:
  - label: Gallery
    name: Open Gallery
    arguments:
      - --gallery
"#,
        )
        .unwrap();
        let desktop = mc.desktop_file(&test_config(), "hola_amigos");
        assert!(desktop.contains("Name=Hola Amigos"));
        assert!(desktop.contains("GenericName=A Flutter Application"));
        assert!(desktop.contains("Exec=hola_amigos %U"));
        assert!(!desktop.contains("LD_LIBRARY_PATH"));
        assert!(desktop.contains("MimeType=audio/mpeg;"));
        assert!(desktop.contains("Categories=Music;Media"));
        assert!(desktop.contains("Keywords=Hello"));
        assert!(desktop.contains("Actions=Gallery"));
        assert!(desktop.contains("[Desktop Action Gallery]"));
        assert!(desktop.contains("Name=Open Gallery"));
        assert!(desktop.contains("Exec=hola_amigos --gallery"));
    }

    #[test]
    fn app_run_script() {
        let script = AppImageMakeConfig::default().app_run(&test_config());
        assert_eq!(
            script,
            "#!/bin/sh\n\
             HERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\n\
             export LD_LIBRARY_PATH=\"$HERE/usr/lib:$HERE/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}\"\n\
             exec \"$HERE/hola_amigos\" \"$@\"\n"
        );
        // Arguments reach the app and the working directory is kept.
        assert!(!script.contains("cd "));
    }

    #[test]
    fn executable_is_the_binary_name() {
        let config = PackageConfig {
            app_binary_name: "hola-amigos".into(),
            ..test_config()
        };
        let mc = AppImageMakeConfig::default();
        assert!(
            mc.app_run(&config)
                .contains("exec \"$HERE/hola-amigos\" \"$@\"\n")
        );
        let desktop = mc.desktop_file(&config, "hola_amigos");
        assert!(desktop.contains("Exec=hola-amigos %U"));
        assert!(desktop.contains("Icon=hola_amigos"));
    }

    #[test]
    fn ldd_output_parsing() {
        let output = "\tlinux-vdso.so.1 (0x00007ffd)\n\tlibkeybinder-3.0.so.0 => /lib64/libkeybinder-3.0.so.0 (0x00007f65)\n\tlibc.so.6 => /lib64/libc.so.6 (0x00007f64)\n\t/lib64/ld-linux-x86-64.so.2 (0x00007f66)\n";
        let deps = parse_ldd_output(output);
        assert_eq!(
            deps.into_iter().collect::<Vec<_>>(),
            vec!["/lib64/libc.so.6", "/lib64/libkeybinder-3.0.so.0"]
        );
    }

    #[test]
    fn failure_output_keeps_stdout() {
        assert_eq!(failure_output(b"", b"boom\n"), "boom");
        assert_eq!(failure_output(b"details\n", b""), "details");
        assert_eq!(
            failure_output(b"error: Media\n", b"ERROR: Desktop file contains errors.\n"),
            "ERROR: Desktop file contains errors.\nerror: Media"
        );
    }

    #[test]
    fn desktop_file_has_a_default_category() {
        let desktop = AppImageMakeConfig::default().desktop_file(&test_config(), "hola_amigos");
        assert!(desktop.contains("Categories=Utility;"));
    }
}
