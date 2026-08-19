use std::path::Path;

use dmg_maker::{CreateOptions, DmgMakerError, create};
use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde_json::json;

/// Builds a macOS `.dmg` using the Rust-native `dmg_maker` crate.
///
/// This replaces the previous implementation that shelled out to `appdmg`
/// (a Node.js tool), removing the Node.js runtime dependency.
pub struct MacOSDmgPackager;

impl AppPackager for MacOSDmgPackager {
    fn name(&self) -> &str {
        "dmg"
    }

    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn package_format(&self) -> &str {
        "dmg"
    }

    #[cfg(not(target_os = "macos"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let pkg_dir = config.packaging_dir();

        // Find the .app bundle, preferring build_output_files over scanning build_output_dir
        let app_bundle = config
            .build_output_files
            .iter()
            .find(|p| p.extension().is_some_and(|x| x == "app"))
            .map(|p| p.to_path_buf())
            .or_else(|| {
                std::fs::read_dir(&config.build_output_dir)
                    .ok()?
                    .filter_map(|e| e.ok())
                    .find(|e| e.path().extension().is_some_and(|x| x == "app"))
                    .map(|e| e.path())
            })
            .ok_or_else(|| PackageError::NotFound(".app bundle in build output".into()))?;

        // Copy the .app into the packaging directory
        run_cp_r(&app_bundle, &pkg_dir)?;

        // Copy the project's dmg packaging assets (background, icon, etc.)
        // These are expected at macos/packaging/dmg/ relative to the project root.
        let dmg_assets = Path::new("macos/packaging/dmg");
        if dmg_assets.exists() {
            run_cp_r_dir_contents(dmg_assets, &pkg_dir)?;
        }

        let output_file = config.output_file();

        // Prefer the project's `macos/packaging/dmg/make_config.yaml` (same
        // appdmg-format schema as Dart's `MakeDmgConfig`: title, icon,
        // background, background-color, icon-size, format, window, code-sign,
        // contents). Fall back to a default spec when it's absent.
        let spec = match load_dmg_make_config(Path::new("macos/packaging/dmg/make_config.yaml"))? {
            Some(spec) => spec,
            None => default_spec(&config.app_name, &pkg_dir),
        };

        // Delegate DMG creation to the native dmg_maker crate.
        create(CreateOptions {
            target: output_file.clone(),
            source: None,
            basepath: Some(pkg_dir.clone()),
            specification: Some(spec),
        })
        .map_err(map_dmg_error)?;

        // Clean up the packaging directory.
        std::fs::remove_dir_all(&pkg_dir).ok();

        Ok(PackageResult {
            artifacts: vec![output_file],
        })
    }
}

/// Loads `macos/packaging/dmg/make_config.yaml` as an appdmg-style JSON spec.
/// Returns `Ok(None)` when the file does not exist.
fn load_dmg_make_config(path: &Path) -> Result<Option<serde_json::Value>, PackageError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackageError::General(format!("Failed to read {}: {}", path.display(), e)))?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| PackageError::General(format!("Failed to parse {}: {}", path.display(), e)))?;
    let spec = serde_json::to_value(yaml).map_err(|e| {
        PackageError::General(format!("Failed to convert {} to JSON: {}", path.display(), e))
    })?;
    Ok(Some(spec))
}

/// The default spec used when no `make_config.yaml` is provided.
/// Paths in the spec are relative to the packaging directory (basepath).
fn default_spec(app_name: &str, pkg_dir: &Path) -> serde_json::Value {
    let escaped_name = app_name.replace('\\', "\\\\").replace('"', "\\\"");
    let mut contents = vec![
        json!({"x": 448, "y": 344, "type": "link", "path": "/Applications"}),
        json!({"x": 192, "y": 344, "type": "file", "path": format!("{escaped_name}.app")}),
    ];

    // Only include a background if background.png actually exists in the
    // packaging dir (from dmg_assets) — dmg_maker errors out trying to
    // copy a background file that was declared but never provided.
    let has_background = pkg_dir.join("background.png").exists();
    if has_background {
        contents.push(json!({"x": 0, "y": 0, "type": "position"}));
    }

    let mut spec = json!({
        "title": escaped_name,
        "icon-size": 80,
        "contents": contents,
    });
    if has_background {
        spec["background"] = json!("background.png");
    }
    spec
}

/// Copy a file or directory recursively.
fn run_cp_r(source: &Path, dest_dir: &Path) -> Result<(), PackageError> {
    let status = std::process::Command::new("cp")
        .args([
            "-RH",
            &source.display().to_string(),
            &dest_dir.display().to_string(),
        ])
        .status()
        .map_err(|e| PackageError::MissingTool(format!("cp: {}", e)))?;
    if !status.success() {
        return Err(PackageError::CommandFailed {
            command: "cp".to_string(),
            stderr: format!(
                "Failed to copy {} to {}",
                source.display(),
                dest_dir.display()
            ),
        });
    }
    Ok(())
}

/// Copy all contents of a source directory into a destination directory.
fn run_cp_r_dir_contents(source: &Path, dest_dir: &Path) -> Result<(), PackageError> {
    let status = std::process::Command::new("cp")
        .args([
            "-RH",
            &format!("{}/.", source.display()),
            &dest_dir.display().to_string(),
        ])
        .status()
        .map_err(|e| PackageError::MissingTool(format!("cp: {}", e)))?;
    if !status.success() {
        return Err(PackageError::CommandFailed {
            command: "cp".to_string(),
            stderr: format!(
                "Failed to copy {} contents to {}",
                source.display(),
                dest_dir.display()
            ),
        });
    }
    Ok(())
}

/// Map `DmgMakerError` to `PackageError`.
fn map_dmg_error(err: DmgMakerError) -> PackageError {
    match err {
        DmgMakerError::UnsupportedPlatform(os) => {
            PackageError::General(format!("DMG creation not supported on {os}"))
        }
        DmgMakerError::TargetExists(path) => {
            PackageError::General(format!("Target already exists: {}", path.display()))
        }
        DmgMakerError::FileNotFound(msg) => PackageError::NotFound(msg),
        DmgMakerError::InvalidConfig(msg) => {
            PackageError::General(format!("Invalid DMG configuration: {msg}"))
        }
        DmgMakerError::CommandFailed { command, stderr } => {
            PackageError::CommandFailed { command, stderr }
        }
        DmgMakerError::Io(e) => PackageError::Io(e),
        DmgMakerError::Json(e) => PackageError::General(format!("JSON error: {e}")),
        DmgMakerError::General(msg) => PackageError::General(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_config_yaml_converts_to_appdmg_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("make_config.yaml");
        std::fs::write(
            &path,
            r#"
title: Hola Amigos
icon: AppIcon.icns
background: background.png
icon-size: 100
window:
  position:
    x: 100
    y: 100
  size:
    width: 600
    height: 400
code-sign:
  signing-identity: "Developer ID Application: Foo"
contents:
  - x: 448
    y: 344
    type: link
    path: /Applications
  - x: 192
    y: 344
    type: file
    path: Hola Amigos.app
"#,
        )
        .unwrap();
        let spec = load_dmg_make_config(&path).unwrap().unwrap();
        assert_eq!(spec["title"], "Hola Amigos");
        assert_eq!(spec["icon-size"], 100);
        assert_eq!(spec["window"]["size"]["width"], 600);
        assert_eq!(
            spec["code-sign"]["signing-identity"],
            "Developer ID Application: Foo"
        );
        assert_eq!(spec["contents"][1]["type"], "file");
    }

    #[test]
    fn missing_make_config_returns_none() {
        assert!(
            load_dmg_make_config(std::path::Path::new("/nonexistent/make_config.yaml"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn default_spec_shape() {
        let dir = tempfile::tempdir().unwrap();
        let spec = default_spec("Demo", dir.path());
        assert_eq!(spec["title"], "Demo");
        assert_eq!(spec["icon-size"], 80);
        assert_eq!(spec["contents"].as_array().unwrap().len(), 2);
        assert!(spec.get("background").is_none());
    }
}
