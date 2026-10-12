use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde_json::json;

use super::common::{FormatVariables, RawPackaging, render_desktop_entry, var};
use super::staging::{self, Contents, Layout};

/// Builds a single-file Flatpak bundle (`.flatpak`) with `flatpak-builder`.
///
/// The application is staged as the contents of `/app`: the bundle in
/// `/app/lib/<binary>`, `/app/bin/<binary>`, and the desktop entry, icons
/// and metainfo named after the app ID, which Flatpak exports. The default
/// manifest runs it on the GNOME runtime, which provides GTK 3. A manifest
/// in `.fastforge/packaging/linux/flatpak/` (`*.yml`, `*.yaml` or
/// `*.json`) replaces the generated one; it can copy the staged tree from
/// `${PACKAGING_DIRECTORY}`.
///
/// Requires `flatpak` and `flatpak-builder`. The runtime and SDK are
/// installed for the user from Flathub when missing.
pub struct LinuxFlatpakPackager;

const FLATHUB: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";
const DEFAULT_RUNTIME: &str = "org.gnome.Platform";
const DEFAULT_SDK: &str = "org.gnome.Sdk";
const DEFAULT_RUNTIME_VERSION: &str = "48";

fn run(cmd: &mut Command) -> Result<(), PackageError> {
    let program = cmd.get_program().to_string_lossy().to_string();
    let out = cmd
        .output()
        .map_err(|e| PackageError::MissingTool(format!("{}: {}", program, e)))?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: program,
            stderr: format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stderr).trim(),
                String::from_utf8_lossy(&out.stdout).trim()
            )
            .trim()
            .to_string(),
        });
    }
    Ok(())
}

/// Flatpak IDs are reverse-DNS names of at least three segments of
/// letters, digits, `_` and `-` (only the last may contain `-`), none
/// starting with a digit.
fn validate_app_id(id: &str) -> Result<(), PackageError> {
    let segments: Vec<&str> = id.split('.').collect();
    let last = segments.len().saturating_sub(1);
    let valid = segments.len() >= 3
        && id.len() <= 255
        && segments.iter().enumerate().all(|(i, segment)| {
            !segment.is_empty()
                && !segment.starts_with(|c: char| c.is_ascii_digit())
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || (c == '-' && i == last))
        });
    if valid {
        Ok(())
    } else {
        Err(PackageError::General(format!(
            "`{}` is not a valid Flatpak app ID. Set `project.app_id` in .fastforge/config.yaml to a reverse-DNS name such as `com.example.App`, matching APPLICATION_ID in linux/CMakeLists.txt.",
            id
        )))
    }
}

/// The generated manifest: the staged `/app` tree copied in by a `simple`
/// module, on the GNOME runtime, with the permissions a Flutter desktop app
/// usually needs.
fn manifest(
    app_id: &str,
    binary: &str,
    app_tree: &Path,
    value: impl Fn(&str, &str) -> String,
) -> String {
    let manifest = json!({
        "id": app_id,
        "runtime": value("FLATPAK_RUNTIME", DEFAULT_RUNTIME),
        "runtime-version": value("FLATPAK_RUNTIME_VERSION", DEFAULT_RUNTIME_VERSION),
        "sdk": value("FLATPAK_SDK", DEFAULT_SDK),
        "command": binary,
        "finish-args": [
            "--share=ipc",
            "--socket=fallback-x11",
            "--socket=wayland",
            "--device=dri",
            "--share=network"
        ],
        "modules": [{
            "name": binary,
            "buildsystem": "simple",
            "build-commands": ["cp -a . /app/"],
            "sources": [{ "type": "dir", "path": app_tree.display().to_string() }]
        }]
    });
    serde_json::to_string_pretty(&manifest).unwrap_or_default()
}

fn default_desktop(config: &PackageConfig, raw: &RawPackaging) -> String {
    let variables = raw.variables();
    render_desktop_entry(&[
        ("Type", Some("Application".to_string())),
        (
            "Name",
            var(variables, "APP_DISPLAY_NAME").or_else(|| Some(config.app_name.clone())),
        ),
        ("Comment", var(variables, "APP_DESCRIPTION")),
        ("Exec", Some(format!("{} %U", config.app_binary_name))),
        ("Icon", Some(raw.app_id().to_string())),
        ("Categories", Some("Utility;".to_string())),
        ("StartupWMClass", Some(raw.app_id().to_string())),
    ])
}

impl AppPackager for LinuxFlatpakPackager {
    fn name(&self) -> &str {
        "flatpak"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        "flatpak"
    }

    #[cfg(not(target_os = "linux"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let pkg_dir = config.packaging_dir()?;
        let output_file = config.output_file()?;
        let binary_name = &config.app_binary_name;
        let layout = Layout::flatpak(binary_name);
        let root = std::path::absolute(&pkg_dir)?;
        let app_tree = root.join("usr");
        let raw = RawPackaging::load(
            config,
            "flatpak",
            FormatVariables {
                package_name: None,
                default_package_name: binary_name.clone(),
                package_version: config.app_version.clone(),
                package_arch: super::common::machine_architecture().to_string(),
                install_dir: Some(format!("/app/lib/{}", binary_name)),
                display_name: None,
                packaging_dir: &app_tree,
                output_file: &output_file,
                extra: vec![],
            },
        )?;
        let result = (|| {
            let app_id = raw.app_id().to_string();
            validate_app_id(&app_id)?;
            staging::stage(
                &raw,
                config,
                &root,
                &layout,
                Contents {
                    icon: raw.icon(None),
                    metainfo: None,
                },
                || default_desktop(config, &raw),
            )?;
            let bin = app_tree.join("bin");
            std::fs::create_dir_all(&bin)?;
            staging::symlink(
                &format!("../lib/{0}/{0}", binary_name),
                &bin.join(binary_name),
            )?;

            let manifest_path = match single_manifest(&raw)? {
                Some(src) => {
                    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("yml");
                    let dest = root.join(format!("{}.{}", app_id, ext));
                    std::fs::write(&dest, raw.render(&src)?)?;
                    dest
                }
                None => {
                    let dest = root.join(format!("{}.json", app_id));
                    std::fs::write(
                        &dest,
                        manifest(&app_id, binary_name, &app_tree, |key, default| {
                            var(raw.variables(), key)
                                .or_else(|| config.env_var(key))
                                .unwrap_or_else(|| default.to_string())
                        }),
                    )?;
                    dest
                }
            };

            // The runtime and SDK come from Flathub, for the current user.
            let mut remote = Command::new("flatpak");
            remote.args([
                "remote-add",
                "--user",
                "--if-not-exists",
                "flathub",
                FLATHUB,
            ]);
            run(&mut remote)?;

            let repo = root.join("repo");
            let mut build = Command::new("flatpak-builder");
            build
                .args([
                    "--user",
                    "--install-deps-from=flathub",
                    "--force-clean",
                    "--disable-rofiles-fuse",
                ])
                .arg(format!(
                    "--state-dir={}",
                    root.join(".flatpak-builder").display()
                ))
                .arg(format!("--repo={}", repo.display()))
                .arg(root.join("build"))
                .arg(&manifest_path);
            raw.apply_env(&mut build);
            run(&mut build)?;

            let mut bundle = Command::new("flatpak");
            bundle
                .args(["build-bundle", &format!("--runtime-repo={}", FLATHUB)])
                .arg(&repo)
                .arg(std::path::absolute(&output_file)?)
                .arg(&app_id);
            run(&mut bundle)
        })();
        std::fs::remove_dir_all(&pkg_dir).ok();
        result?;
        config.resolve_result(output_file)
    }
}

/// The raw manifest, if the format directory has exactly one.
fn single_manifest(raw: &RawPackaging) -> Result<Option<std::path::PathBuf>, PackageError> {
    let mut found = None;
    for ext in ["yml", "yaml", "json"] {
        if let Some(path) = raw.file_with_extension(ext)? {
            if found.is_some() {
                return Err(PackageError::General(
                    "Several Flatpak manifests in .fastforge/packaging/linux/flatpak/; keep one."
                        .into(),
                ));
            }
            found = Some(path);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packages a stand-in bundle (a shell script as the binary) with the
    /// real flatpak-builder, then installs the bundle and runs it.
    /// Set `FLATPAK_RUNTIME`/`FLATPAK_SDK`/`FLATPAK_RUNTIME_VERSION` to use a
    /// smaller runtime.
    #[test]
    #[ignore = "requires flatpak and flatpak-builder and downloads a runtime"]
    #[cfg(target_os = "linux")]
    fn builds_installs_and_runs_a_flatpak() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("bundle");
        std::fs::create_dir_all(bundle.join("data")).unwrap();
        std::fs::write(bundle.join("data/asset.txt"), "asset").unwrap();
        let binary = bundle.join("my_app");
        std::fs::write(
            &binary,
            "#!/bin/sh\nhere=$(dirname \"$(readlink -f \"$0\")\")\necho \"flatpak-smoke:$(cat \"$here/data/asset.txt\")\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".fastforge")).unwrap();
        std::fs::write(
            project.join(".fastforge/config.yaml"),
            "project:\n  app_id: dev.fastforge.FlatpakSmoke\n  display_name: Flatpak Smoke\n",
        )
        .unwrap();
        std::env::set_current_dir(&project).unwrap();
        let config = PackageConfig {
            app_name: "my_app".into(),
            app_binary_name: "my_app".into(),
            app_version: "1.0.0+1".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "flatpak".into(),
            is_installer: false,
            build_output_dir: bundle,
            build_output_files: vec![],
            output_dir: project.join("dist"),
            environment: Default::default(),
        };
        let result = LinuxFlatpakPackager.package(&config).unwrap();
        let artifact = &result.artifacts[0];
        assert!(artifact.is_file());
        let status = Command::new("flatpak")
            .args(["install", "--user", "-y", "--noninteractive", "--bundle"])
            .arg(artifact)
            .status()
            .unwrap();
        assert!(status.success());
        let out = Command::new("flatpak")
            .args(["run", "--user", "dev.fastforge.FlatpakSmoke"])
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("flatpak-smoke:asset"),
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn app_ids_follow_flatpak_rules() {
        for ok in [
            "com.example.App",
            "dev.fastforge.hello_world",
            "org.example.my-app",
        ] {
            assert!(validate_app_id(ok).is_ok(), "{ok}");
        }
        for bad in [
            "hello_world",
            "com.example",
            "com.1example.App",
            "com.my-org.App",
            "com..App",
        ] {
            assert!(validate_app_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn manifest_copies_the_staged_tree_on_the_gnome_runtime() {
        let manifest: serde_json::Value = serde_json::from_str(&manifest(
            "com.example.App",
            "my_app",
            Path::new("/tmp/stage/usr"),
            |key, default| match key {
                "FLATPAK_RUNTIME_VERSION" => "47".to_string(),
                _ => default.to_string(),
            },
        ))
        .unwrap();
        assert_eq!(manifest["runtime"], "org.gnome.Platform");
        assert_eq!(manifest["runtime-version"], "47");
        assert_eq!(manifest["command"], "my_app");
        assert_eq!(
            manifest["modules"][0]["sources"][0]["path"],
            "/tmp/stage/usr"
        );
        assert_eq!(manifest["modules"][0]["build-commands"][0], "cp -a . /app/");
    }
}
