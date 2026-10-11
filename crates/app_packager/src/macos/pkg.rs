use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

/// Builds a macOS `.pkg` installer using `xcrun productbuild` (and optionally
/// `productsign`), mirroring Dart's `AppPackageMakerPkg`.
///
/// Requires Xcode command-line tools.
#[derive(Default)]
pub struct MacOSPkgPackager {
    /// Optional code-signing identity (e.g. `"Developer ID Installer: ..."`)
    pub sign_identity: Option<String>,
    /// Installation path prefix (defaults to `/Applications/`)
    pub install_path: Option<String>,
    /// Optional path to a directory containing pre/post-install scripts.
    pub scripts: Option<String>,
}

impl MacOSPkgPackager {
    /// Load configuration from a YAML file (e.g. `macos/packaging/pkg/make_config.yaml`).
    /// Returns `Self::default()` if the file does not exist.
    pub fn from_yaml_file(path: &Path) -> Result<Self, PackageError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            PackageError::General(format!("Failed to read {}: {}", path.display(), e))
        })?;
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct Config {
            install_path: Option<String>,
            sign_identity: Option<String>,
            scripts: Option<String>,
        }
        let cfg: Config = serde_yaml::from_str(&content).map_err(|e| {
            PackageError::General(format!("Failed to parse {}: {}", path.display(), e))
        })?;
        Ok(Self {
            sign_identity: cfg.sign_identity,
            install_path: cfg.install_path,
            scripts: cfg.scripts,
        })
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

/// `<name>-unsigned.pkg` next to the output file.
fn unsigned_pkg_path(output_file: &Path) -> PathBuf {
    let stem = output_file
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    output_file.with_file_name(format!("{}-unsigned.pkg", stem))
}

/// `xcrun` arguments for productbuild, in Dart's order: `--scripts` is
/// appended after the output path.
fn productbuild_args(
    app_path: &Path,
    install_path: Option<&str>,
    unsigned_path: &Path,
    scripts: Option<&str>,
) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        "productbuild".into(),
        "--component".into(),
        app_path.into(),
        install_path.unwrap_or("/Applications/").into(),
        unsigned_path.into(),
    ];
    if let Some(scripts) = scripts {
        args.push("--scripts".into());
        args.push(scripts.into());
    }
    args
}

/// Injects `<domains>` before the first `<options ` element of the
/// Distribution file (Dart's `replaceFirst`).
fn inject_domains(distribution: &str) -> String {
    distribution.replacen(
        "<options ",
        "<domains enable_local=\"true\" enable_currentUserHome=\"false\" enable_anywhere=\"false\" />\n    <options ",
        1,
    )
}

/// Removes the relocation / bundle-upgrade elements from a component
/// PackageInfo, mirroring Dart's regex replacements:
/// `<relocate>.*?</relocate>`, `<upgrade-bundle>.*?</upgrade-bundle>`,
/// `<update-bundle\s*/>`, `<atomic-update-bundle\s*/>` and
/// `<strict-identifier>.*?</strict-identifier>`.
fn strip_relocation(package_info: &str) -> String {
    let mut content = package_info.to_string();
    for tag in ["relocate", "upgrade-bundle"] {
        content = remove_elements(&content, &format!("<{}>", tag), &format!("</{}>", tag));
    }
    for tag in ["update-bundle", "atomic-update-bundle"] {
        content = remove_empty_elements(&content, tag);
    }
    remove_elements(&content, "<strict-identifier>", "</strict-identifier>")
}

/// Removes every non-greedy `open ... close` span (across lines).
fn remove_elements(content: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find(open) {
        let Some(end) = rest[start + open.len()..].find(close) else {
            break;
        };
        out.push_str(&rest[..start]);
        rest = &rest[start + open.len() + end + close.len()..];
    }
    out.push_str(rest);
    out
}

/// Removes every self-closing `<tag\s*/>` element.
fn remove_empty_elements(content: &str, tag: &str) -> String {
    let open = format!("<{}", tag);
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        let trimmed = after.trim_start();
        if let Some(tail) = trimmed.strip_prefix("/>") {
            out.push_str(&rest[..start]);
            rest = tail;
        } else {
            out.push_str(&rest[..start + open.len()]);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

impl AppPackager for MacOSPkgPackager {
    fn name(&self) -> &str {
        "pkg"
    }

    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn package_format(&self) -> &str {
        "pkg"
    }

    #[cfg(not(target_os = "macos"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        // Flutter's macOS build exposes the .app bundle as the first entry in
        // build_output_files. Use productbuild's component mode so the bundle
        // is installed as /Applications/<App>.app instead of flattening the
        // app bundle contents into /Applications.
        let app_path = config
            .first_build_output_file()
            .ok_or_else(|| PackageError::General("no build output files".into()))?;

        let output_file = config.output_file()?;
        let unsigned_path = unsigned_pkg_path(&output_file);

        run(Command::new("xcrun").args(productbuild_args(
            app_path,
            self.install_path.as_deref(),
            &unsigned_path,
            self.scripts.as_deref(),
        )))?;

        // Fix the pkg metadata (expand → edit → flatten), mirroring Dart.
        // `productbuild --component` output has two problems:
        // 1. Distribution lacks <domains>, so the GUI installer may pick the
        //    wrong install location.
        // 2. The component PackageInfo contains <relocate>, which redirects
        //    the install to an already existing copy of the bundle (e.g. the
        //    build output).
        let expand_dir = {
            let mut p = unsigned_path.clone().into_os_string();
            p.push(".expanded");
            std::path::PathBuf::from(p)
        };
        if expand_dir.exists() {
            std::fs::remove_dir_all(&expand_dir)?;
        }
        run(Command::new("pkgutil").args([
            "--expand".as_ref(),
            unsigned_path.as_os_str(),
            expand_dir.as_os_str(),
        ]))?;

        let distribution = expand_dir.join("Distribution");
        if distribution.exists() {
            let content = std::fs::read_to_string(&distribution)?;
            std::fs::write(&distribution, inject_domains(&content))?;
        }
        // The inner component packages are already expanded directories;
        // edit their PackageInfo in place.
        for entry in std::fs::read_dir(&expand_dir)? {
            let path = entry?.path();
            if !path.is_dir() || path.extension().is_none_or(|e| e != "pkg") {
                continue;
            }
            let package_info = path.join("PackageInfo");
            if package_info.exists() {
                let content = std::fs::read_to_string(&package_info)?;
                std::fs::write(&package_info, strip_relocation(&content))?;
            }
        }

        std::fs::remove_file(&unsigned_path)?;
        run(Command::new("pkgutil").args([
            "--flatten".as_ref(),
            expand_dir.as_os_str(),
            unsigned_path.as_os_str(),
        ]))?;
        std::fs::remove_dir_all(&expand_dir)?;

        if let Some(identity) = &self.sign_identity {
            run(Command::new("xcrun").args([
                "productsign".as_ref(),
                "--sign".as_ref(),
                identity.as_ref(),
                unsigned_path.as_os_str(),
                output_file.as_os_str(),
            ]))?;
            std::fs::remove_file(&unsigned_path)?;
        } else {
            std::fs::rename(&unsigned_path, &output_file)?;
        }

        config.resolve_result(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_are_appended_after_output_like_dart() {
        let args = productbuild_args(
            Path::new("build/Demo.app"),
            None,
            Path::new("dist/demo-unsigned.pkg"),
            Some("macos/packaging/pkg/scripts"),
        );
        let args: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "productbuild",
                "--component",
                "build/Demo.app",
                "/Applications/",
                "dist/demo-unsigned.pkg",
                "--scripts",
                "macos/packaging/pkg/scripts",
            ]
        );
    }

    #[test]
    fn unsigned_path_is_next_to_output() {
        assert_eq!(
            unsigned_pkg_path(Path::new("dist/1.0.0+1/demo-1.0.0+1-macos.pkg")),
            PathBuf::from("dist/1.0.0+1/demo-1.0.0+1-macos-unsigned.pkg")
        );
    }

    #[test]
    fn distribution_gets_domains_before_first_options() {
        let input = "<installer-gui-script>\n    <options customize=\"never\"/>\n    <options x=\"y\"/>\n</installer-gui-script>";
        let out = inject_domains(input);
        assert_eq!(
            out,
            "<installer-gui-script>\n    <domains enable_local=\"true\" enable_currentUserHome=\"false\" enable_anywhere=\"false\" />\n    <options customize=\"never\"/>\n    <options x=\"y\"/>\n</installer-gui-script>"
        );
    }

    #[test]
    fn package_info_relocation_elements_are_stripped() {
        let input = r#"<pkg-info format-version="2" identifier="com.example.demo">
    <payload numberOfFiles="10" installKBytes="100"/>
    <bundle-version>
        <bundle id="com.example.demo" path="./Demo.app"/>
    </bundle-version>
    <upgrade-bundle>
        <bundle id="com.example.demo"/>
    </upgrade-bundle>
    <update-bundle/>
    <atomic-update-bundle />
    <strict-identifier>
        <bundle id="com.example.demo"/>
    </strict-identifier>
    <relocate>
        <bundle id="com.example.demo"/>
    </relocate>
</pkg-info>"#;
        let out = strip_relocation(input);
        assert!(!out.contains("relocate"));
        assert!(!out.contains("upgrade-bundle"));
        assert!(!out.contains("update-bundle"));
        assert!(!out.contains("strict-identifier"));
        assert!(out.contains("<bundle-version>"));
        assert!(out.contains("<payload numberOfFiles=\"10\""));
    }

    #[test]
    fn non_empty_update_bundle_is_kept() {
        // Only the self-closing form matches Dart's `<update-bundle\s*/>`.
        assert_eq!(
            remove_empty_elements("<update-bundle-x/><update-bundle >a", "update-bundle"),
            "<update-bundle-x/><update-bundle >a"
        );
    }
}
