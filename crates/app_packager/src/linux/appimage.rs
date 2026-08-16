use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

use super::common::{load_make_config, uname_machine};

/// Builds a Linux AppImage using `appimagetool`, mirroring Dart's
/// `AppPackageMakerAppImage`.
///
/// Reads `linux/packaging/appimage/make_config.yaml` when present (same
/// schema as Dart's `MakeAppImageConfig`); falls back to sensible defaults
/// otherwise.
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
    pub startup_wm_class: Option<String>,
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
    fn desktop_file(&self, config: &PackageConfig) -> String {
        let app_name = &config.app_name;
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
            (
                "Exec",
                format!("LD_LIBRARY_PATH=usr/lib {} %u", app_name),
            ),
            ("Icon", app_name.clone()),
            ("Type", "Application".to_string()),
            (
                "StartupNotify",
                self.startup_notify.unwrap_or(false).to_string(),
            ),
        ];
        if let Some(wm) = self
            .startup_wm_class
            .as_ref()
            .filter(|s| !s.is_empty())
        {
            fields.push(("StartupWMClass", wm.clone()));
        }
        if let Some(mime) = self
            .supported_mime_type
            .as_ref()
            .filter(|v| !v.is_empty())
        {
            fields.push(("MimeType", format!("{};", mime.join(";"))));
        }
        if !self.categories.is_empty() {
            fields.push(("Categories", self.categories.join(";")));
        }
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
                format!(
                    "[Desktop Action {}]\nName={}\nExec=LD_LIBRARY_PATH=usr/lib {} {} %u",
                    action.label,
                    action.name,
                    app_name,
                    action.arguments.join(" "),
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!("[Desktop Entry]\n{}\n\n{}", entry, actions)
    }

    /// Renders the `AppRun` launcher script, mirroring Dart's `appRunContent`.
    fn app_run(&self, config: &PackageConfig) -> String {
        format!(
            "#!/bin/bash\n\ncd \"$(dirname \"$0\")\"\nexport LD_LIBRARY_PATH=usr/lib\nexec ./{}\n",
            config.app_name
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
            stderr: String::from_utf8_lossy(&out.stderr).into(),
        });
    }
    Ok(())
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

        // Copy flutter build output contents into AppDir
        run(Command::new("cp").args([
            "-r",
            &format!("{}/.", config.build_output_dir.display()),
            &app_dir.display().to_string(),
        ]))?;

        // Write .desktop file and AppRun
        std::fs::write(
            app_dir.join(format!("{}.desktop", app_name)),
            make_config.desktop_file(config),
        )?;
        let app_run_path = app_dir.join("AppRun");
        std::fs::write(&app_run_path, make_config.app_run(config))?;
        run(Command::new("chmod").args(["+x", &app_run_path.display().to_string()]))?;

        // Install the configured icon at AppDir root and in hicolor dirs
        if let Some(icon) = &make_config.icon {
            let icon_path = Path::new(icon);
            if !icon_path.exists() {
                return Err(PackageError::NotFound(format!(
                    "icon {} path doesn't exist",
                    icon
                )));
            }
            let ext = icon_path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            std::fs::copy(icon_path, app_dir.join(format!("{}{}", app_name, ext)))?;
            for size in ["128x128", "256x256"] {
                let dir = app_dir
                    .join("usr/share/icons/hicolor")
                    .join(size)
                    .join("apps");
                std::fs::create_dir_all(&dir)?;
                std::fs::copy(icon_path, dir.join(format!("{}{}", app_name, ext)))?;
            }
        }

        // Install the configured metainfo
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
            let metainfo_dir = app_dir.join("usr/share/metainfo");
            std::fs::create_dir_all(&metainfo_dir)?;
            std::fs::copy(
                metainfo_path,
                metainfo_dir.join(format!("{}{}", config.app_binary_name, ext)),
            )?;
        }

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
        let arch = if uname_machine() == "aarch64" {
            "aarch64"
        } else {
            "x86_64"
        };
        run(Command::new("appimagetool")
            .args([
                "--no-appstream",
                &app_dir.display().to_string(),
                &output_file.display().to_string(),
            ])
            .env("ARCH", arch))?;

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
            package_format: "appimage".into(),
            is_installer: false,
            build_output_dir: PathBuf::new(),
            build_output_files: vec![],
            output_dir: PathBuf::new(),
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
startup_wm_class: com.example.hola_amigos
actions:
  - label: Gallery
    name: Open Gallery
    arguments:
      - --gallery
"#,
        )
        .unwrap();
        let desktop = mc.desktop_file(&test_config());
        assert!(desktop.contains("Name=Hola Amigos"));
        assert!(desktop.contains("GenericName=A Flutter Application"));
        assert!(desktop.contains("Exec=LD_LIBRARY_PATH=usr/lib hola_amigos %u"));
        assert!(desktop.contains("StartupWMClass=com.example.hola_amigos"));
        assert!(desktop.contains("MimeType=audio/mpeg;"));
        assert!(desktop.contains("Categories=Music;Media"));
        assert!(desktop.contains("Keywords=Hello"));
        assert!(desktop.contains("Actions=Gallery"));
        assert!(desktop.contains("[Desktop Action Gallery]"));
        assert!(desktop.contains("Name=Open Gallery"));
        assert!(desktop.contains("Exec=LD_LIBRARY_PATH=usr/lib hola_amigos --gallery %u"));
    }

    #[test]
    fn desktop_file_omits_empty_startup_wm_class() {
        let mc: AppImageMakeConfig = serde_yaml::from_str(
            r#"
display_name: Hola Amigos
icon: assets/logo.png
startup_wm_class: ""
"#,
        )
        .unwrap();
        let desktop = mc.desktop_file(&test_config());
        assert!(!desktop.contains("StartupWMClass"));
    }

    #[test]
    fn app_run_script() {
        let script = AppImageMakeConfig::default().app_run(&test_config());
        assert!(script.starts_with("#!/bin/bash"));
        assert!(script.contains("export LD_LIBRARY_PATH=usr/lib"));
        assert!(script.contains("exec ./hola_amigos"));
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
}
