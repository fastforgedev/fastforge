use std::path::Path;

use dmg_maker::{CreateOptions, DmgMakerError, create};
use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde_json::json;

use super::notarize;

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
        let pkg_dir = config.packaging_dir()?;
        // Dart deletes the packaging directory in a `finally` block, so it is
        // cleaned up even when DMG creation fails.
        let result = make_dmg(config, &pkg_dir);
        std::fs::remove_dir_all(&pkg_dir).ok();
        let output_file = result?;
        config.resolve_result(output_file)
    }
}

/// Stages the `.app` and packaging assets in `pkg_dir` and creates the DMG,
/// returning the output path.
fn make_dmg(config: &PackageConfig, pkg_dir: &Path) -> Result<std::path::PathBuf, PackageError> {
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

    // Prefer the project's `make_config.yaml` (same appdmg-format schema as
    // Dart's `MakeDmgConfig`: title, icon, background, background-color,
    // icon-size, format, window, code-sign, contents). It lives in
    // `<packaging root>/dmg/`, `macos/packaging/dmg/` for Flutter projects
    // (see `PackageConfig::packaging_root`). Fall back to a default spec
    // when it's absent.
    let dmg_assets = config.format_packaging_dir("dmg");
    let make_config = dmg_assets.join("make_config.yaml");
    let mut spec = load_dmg_make_config(&make_config)?;
    let configured = match spec.as_mut() {
        Some(spec) => take_notarize(spec, &make_config)?,
        None => false,
    };
    let credentials = notarize::enabled(configured, config)
        .then(|| notarize::Credentials::from_config(config))
        .transpose()?;

    // The app is stapled before it goes into the image, so the DMG ships a
    // stapled app that passes Gatekeeper offline.
    if let Some(credentials) = &credentials {
        notarize::notarize_app(&app_bundle, credentials, pkg_dir)?;
    }

    // Copy the .app into the packaging directory
    run_cp_r(&app_bundle, pkg_dir)?;

    // Xcode names the bundle after `PRODUCT_NAME`, which does not have to match
    // the pubspec name, so the spec has to point at the bundle that was copied
    // rather than at `<app name>.app`.
    let bundle_file_name = app_bundle
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("{}.app", config.app_name));

    // Copy the project's dmg packaging assets (background, icon, etc.) from
    // the directory that holds `make_config.yaml`.
    if dmg_assets.exists() {
        run_cp_r_dir_contents(&dmg_assets, pkg_dir)?;
    }

    let output_file = config.output_file()?;

    let mut spec =
        spec.unwrap_or_else(|| default_spec(&config.app_name, &bundle_file_name, pkg_dir));
    check_app_entries(&spec, pkg_dir, &bundle_file_name, &make_config)?;
    if credentials.is_some() {
        sign_for_notarization(&mut spec, &app_bundle, &make_config)?;
    }

    // Delegate DMG creation to the native dmg_maker crate.
    create(CreateOptions {
        target: output_file.clone(),
        source: None,
        basepath: Some(pkg_dir.to_path_buf()),
        specification: Some(spec),
    })
    .map_err(map_dmg_error)?;

    if let Some(credentials) = &credentials {
        notarize::notarize_and_staple(&output_file, credentials)?;
    }
    Ok(output_file)
}

/// Removes Fastforge's `notarize` key, which is not part of the appdmg spec
/// `dmg_maker` validates.
fn take_notarize(spec: &mut serde_json::Value, make_config: &Path) -> Result<bool, PackageError> {
    let Some(value) = spec
        .as_object_mut()
        .and_then(|spec| spec.remove("notarize"))
    else {
        return Ok(false);
    };
    value.as_bool().ok_or_else(|| {
        PackageError::General(format!(
            "`notarize` in {} must be true or false",
            make_config.display()
        ))
    })
}

/// A notarized DMG has to be signed with a secure timestamp. Without a
/// `code-sign` entry, the image is signed with the app's Developer ID
/// Application identity.
fn sign_for_notarization(
    spec: &mut serde_json::Value,
    app_bundle: &Path,
    make_config: &Path,
) -> Result<(), PackageError> {
    if spec.get("code-sign").is_none_or(|sign| sign.is_null()) {
        let identity = notarize::developer_id_identity(app_bundle).ok_or_else(|| {
            PackageError::General(format!(
                "Notarizing the DMG needs a signing identity: add `code-sign.signing-identity` \
                 to {}, or sign {} with a \"Developer ID Application\" identity.",
                make_config.display(),
                app_bundle.display()
            ))
        })?;
        spec["code-sign"] = json!({ "signing-identity": identity });
    }
    if let Some(sign) = spec["code-sign"].as_object_mut() {
        sign.insert("timestamp".into(), json!(true));
    }
    Ok(())
}

/// Fails early, naming the bundle the build produced, when the spec places a
/// `.app` that is not in the packaging directory: Xcode names the bundle
/// after `PRODUCT_NAME`, which often differs from the name a spec assumes.
fn check_app_entries(
    spec: &serde_json::Value,
    pkg_dir: &Path,
    bundle_file_name: &str,
    make_config: &Path,
) -> Result<(), PackageError> {
    let entries = spec["contents"].as_array().into_iter().flatten();
    for entry in entries {
        let Some(path) = entry["path"].as_str() else {
            continue;
        };
        let is_app = Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"));
        if entry["type"] == "file" && is_app && !pkg_dir.join(path).exists() {
            return Err(PackageError::NotFound(format!(
                "`{}` from {} is not in the build output; the build produced `{}`. \
                 Set the entry's `path` to `{}`.",
                path,
                make_config.display(),
                bundle_file_name,
                bundle_file_name
            )));
        }
    }
    Ok(())
}

/// Loads the DMG `make_config.yaml` as an appdmg-style JSON spec.
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
        PackageError::General(format!(
            "Failed to convert {} to JSON: {}",
            path.display(),
            e
        ))
    })?;
    Ok(Some(spec))
}

/// The default spec used when no `make_config.yaml` is provided.
/// Paths in the spec are relative to the packaging directory (basepath).
///
/// `app_name` titles the disk image; `bundle_file_name` is the `.app` that was
/// copied into the packaging directory, which Xcode names after `PRODUCT_NAME`
/// and not after the pubspec.
fn default_spec(app_name: &str, bundle_file_name: &str, pkg_dir: &Path) -> serde_json::Value {
    let escaped_name = app_name.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_bundle = bundle_file_name.replace('\\', "\\\\").replace('"', "\\\"");
    let mut contents = vec![
        json!({"x": 448, "y": 344, "type": "link", "path": "/Applications"}),
        json!({"x": 192, "y": 344, "type": "file", "path": escaped_bundle}),
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
    fn app_entries_must_name_the_built_bundle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("My App.app")).unwrap();
        let spec = |path: &str| {
            json!({ "title": "t", "contents": [
                { "x": 1, "y": 1, "type": "link", "path": "/Applications" },
                { "x": 2, "y": 2, "type": "file", "path": path },
            ]})
        };
        let make_config = Path::new("macos/packaging/dmg/make_config.yaml");
        assert!(
            check_app_entries(&spec("My App.app"), dir.path(), "My App.app", make_config).is_ok()
        );
        let error = check_app_entries(&spec("my_app.app"), dir.path(), "My App.app", make_config)
            .unwrap_err()
            .to_string();
        assert!(error.contains("`my_app.app`") && error.contains("produced `My App.app`"));
        assert!(error.contains("macos/packaging/dmg/make_config.yaml"));
    }

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
    fn notarize_key_is_taken_out_of_the_spec() {
        let make_config = Path::new("packaging/dmg/make_config.yaml");
        let mut spec = json!({ "title": "t", "contents": [], "notarize": true });
        assert!(take_notarize(&mut spec, make_config).unwrap());
        assert!(spec.get("notarize").is_none());
        assert!(!take_notarize(&mut spec, make_config).unwrap());
        let mut invalid = json!({ "title": "t", "notarize": "yes" });
        let error = take_notarize(&mut invalid, make_config)
            .unwrap_err()
            .to_string();
        assert!(error.contains("packaging/dmg/make_config.yaml"));
    }

    #[test]
    fn notarized_dmg_is_signed_with_a_timestamp() {
        let mut spec = json!({ "title": "t", "contents": [],
            "code-sign": { "signing-identity": "Developer ID Application: Foo (TEAM)" } });
        let make_config = Path::new("macos/packaging/dmg/make_config.yaml");
        sign_for_notarization(&mut spec, Path::new("/nonexistent/Demo.app"), make_config).unwrap();
        assert_eq!(spec["code-sign"]["timestamp"], true);
        assert_eq!(
            spec["code-sign"]["signing-identity"],
            "Developer ID Application: Foo (TEAM)"
        );

        // Without `code-sign`, an unsigned app leaves no identity to use.
        let mut unsigned = json!({ "title": "t", "contents": [] });
        let error = sign_for_notarization(
            &mut unsigned,
            Path::new("/nonexistent/Demo.app"),
            make_config,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("code-sign.signing-identity"));
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
        let spec = default_spec("Demo", "Demo.app", dir.path());
        assert_eq!(spec["title"], "Demo");
        assert_eq!(spec["icon-size"], 80);
        assert_eq!(spec["contents"].as_array().unwrap().len(), 2);
        assert!(spec.get("background").is_none());
    }

    #[test]
    fn default_spec_points_at_the_copied_bundle() {
        let dir = tempfile::tempdir().unwrap();
        // Xcode's `PRODUCT_NAME` (`beyondtranslate`) need not match the pubspec
        // name (`beyondtranslate_desktop`).
        let spec = default_spec("beyondtranslate_desktop", "beyondtranslate.app", dir.path());
        assert_eq!(spec["contents"][1]["path"], "beyondtranslate.app");
        assert_eq!(spec["title"], "beyondtranslate_desktop");
    }
}
