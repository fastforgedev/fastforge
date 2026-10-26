use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

use super::notarize;

/// Builds a macOS `.pkg` installer using `xcrun productbuild` (and optionally
/// `productsign`), mirroring Dart's `AppPackageMakerPkg`.
///
/// Two kinds of package come out of it:
///
/// - A direct-download installer (the default): productbuild's output is
///   expanded, its Distribution and PackageInfo are fixed up, and the result
///   is flattened, optionally signed with `productsign` and notarized.
/// - A Mac App Store package (`app_store`): the untouched output of a single
///   `productbuild --component ... --sign` run. App Store Connect rejects
///   rewritten packages (ITMS-90229 "not declared in PackageInfo",
///   ITMS-90270 "Unsupported toolchain"), so nothing else touches it.
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
    /// Notarize and staple the signed package.
    pub notarize: bool,
    /// Build a Mac App Store package: productbuild's output signed in one
    /// step and left as is. Needs `sign_identity`; `scripts` and `notarize`
    /// do not apply.
    pub app_store: bool,
}

/// Prefixes of the installer identities Apple issues for Mac App Store
/// submissions; a `sign-identity` starting with one of them switches the
/// packager to App Store mode unless `app-store: false` is set.
const APP_STORE_INSTALLER_IDENTITY_PREFIXES: [&str; 2] = [
    "3rd Party Mac Developer Installer",
    "Mac Installer Distribution",
];

/// Whether `identity` names a Mac App Store installer certificate (as opposed
/// to a "Developer ID Installer" one).
pub fn is_app_store_installer_identity(identity: &str) -> bool {
    let identity = identity.trim();
    APP_STORE_INSTALLER_IDENTITY_PREFIXES
        .iter()
        .any(|prefix| identity.starts_with(prefix))
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
            #[serde(default)]
            notarize: bool,
            /// Explicit setting; unset means "detect from `sign-identity`".
            app_store: Option<bool>,
        }
        let cfg: Config = serde_yaml::from_str(&content).map_err(|e| {
            PackageError::General(format!("Failed to parse {}: {}", path.display(), e))
        })?;
        let app_store = cfg.app_store.unwrap_or_else(|| {
            cfg.sign_identity
                .as_deref()
                .is_some_and(is_app_store_installer_identity)
        });
        Ok(Self {
            sign_identity: cfg.sign_identity,
            install_path: cfg.install_path,
            scripts: cfg.scripts,
            notarize: cfg.notarize,
            app_store,
        })
    }

    /// Checks the App Store mode settings, returning the installer identity.
    fn app_store_identity(&self, config: &PackageConfig) -> Result<&str, PackageError> {
        let identity = self.sign_identity.as_deref().ok_or_else(|| {
            PackageError::General(
                "A Mac App Store package needs `sign-identity` (a \"3rd Party Mac Developer \
                 Installer\" or \"Mac Installer Distribution\" identity) in \
                 macos/packaging/pkg/make_config.yaml, or `app-store: false` for a direct-\
                 download installer"
                    .into(),
            )
        })?;
        if self.scripts.is_some() {
            return Err(PackageError::General(
                "`scripts` cannot be used with `app-store: true` in \
                 macos/packaging/pkg/make_config.yaml: Mac App Store packages cannot carry \
                 install scripts. Remove `scripts`, or set `app-store: false` for a \
                 direct-download installer"
                    .into(),
            ));
        }
        if notarize::enabled(self.notarize, config) {
            return Err(PackageError::General(
                "`notarize` cannot be used with `app-store: true` in \
                 macos/packaging/pkg/make_config.yaml: Mac App Store packages are not \
                 notarized (App Store Connect handles them). Remove `notarize` (or \
                 `--notarize`), or set `app-store: false` for a direct-download installer"
                    .into(),
            ));
        }
        Ok(identity)
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

/// `xcrun` arguments for the single productbuild run of App Store mode: the
/// component is signed by productbuild itself, so the output needs no
/// `productsign` pass and is not rewritten. This is the command
/// `xcodebuild -exportArchive` runs for the `app-store-connect` method
/// (`productbuild --sign <identity> --component <App.app> /Applications
/// <out.pkg>`, see its Packaging.log); Xcode passes the install path without
/// a trailing slash, so the slash is dropped here too and the resulting
/// `install-location` matches Xcode's byte for byte.
fn app_store_productbuild_args(
    app_path: &Path,
    install_path: Option<&str>,
    identity: &str,
    output_file: &Path,
) -> Vec<OsString> {
    let install_path = install_path.unwrap_or("/Applications");
    let trimmed = install_path.trim_end_matches('/');
    let install_path = if trimmed.is_empty() { "/" } else { trimmed };
    vec![
        "productbuild".into(),
        "--component".into(),
        app_path.into(),
        install_path.into(),
        "--sign".into(),
        identity.into(),
        output_file.into(),
    ]
}

/// `xcrun productsign` arguments; a package that is notarized needs a secure
/// timestamp.
fn productsign_args(
    identity: &str,
    timestamp: bool,
    unsigned_path: &Path,
    output_file: &Path,
) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["productsign".into(), "--sign".into(), identity.into()];
    if timestamp {
        args.push("--timestamp".into());
    }
    args.push(unsigned_path.into());
    args.push(output_file.into());
    args
}

/// Injects `<domains>` before the first `<options ` element of the
/// Distribution file (Dart's `replaceFirst`), limiting the install to the
/// startup disk. Dart writes `enable_local`, which Installer does not know:
/// depending on the macOS version it then offers every volume or none but
/// the startup disk.
fn inject_domains(distribution: &str) -> String {
    distribution.replacen(
        "<options ",
        "<domains enable_localSystem=\"true\" enable_currentUserHome=\"false\" enable_anywhere=\"false\" />\n    <options ",
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

        if self.app_store {
            // App Store Connect validates the package against what productbuild
            // wrote (ITMS-90229, ITMS-90270), so hand over its output untouched:
            // one run that also signs, no expand/edit/flatten, no productsign.
            // The component PackageInfo then lists the app bundle only, exactly
            // like Xcode's own export; it is the rewrite that the store refuses,
            // not missing declarations of nested bundles.
            let identity = self.app_store_identity(config)?;
            let output_file = config.output_file()?;
            run(Command::new("xcrun").args(app_store_productbuild_args(
                app_path,
                self.install_path.as_deref(),
                identity,
                &output_file,
            )))?;
            return config.resolve_result(output_file);
        }

        // Fail before building anything the notary service would reject.
        let credentials = if notarize::enabled(self.notarize, config) {
            if self.sign_identity.is_none() {
                return Err(PackageError::General(
                    "Notarizing the PKG needs `sign-identity` (a \"Developer ID Installer\" \
                     identity) in macos/packaging/pkg/make_config.yaml"
                        .into(),
                ));
            }
            let credentials = notarize::Credentials::from_config(config)?;
            notarize::check_app_signature(app_path)?;
            Some(credentials)
        } else {
            None
        };

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
            run(Command::new("xcrun").args(productsign_args(
                identity,
                credentials.is_some(),
                &unsigned_path,
                &output_file,
            )))?;
            std::fs::remove_file(&unsigned_path)?;
        } else {
            std::fs::rename(&unsigned_path, &output_file)?;
        }

        if let Some(credentials) = &credentials {
            notarize::notarize_and_staple(&output_file, credentials)?;
        }

        config.resolve_result(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    fn test_config() -> PackageConfig {
        PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "demo".into(),
            app_version: "1.0.0+1".into(),
            build_mode: "release".into(),
            platform: Platform::MacOS,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "pkg".into(),
            is_installer: true,
            build_output_dir: PathBuf::from("build/macos/Build/Products/Release"),
            build_output_files: vec![PathBuf::from("build/macos/Build/Products/Release/Demo.app")],
            output_dir: PathBuf::from("dist"),
            environment: Default::default(),
        }
    }

    fn from_yaml(content: &str) -> MacOSPkgPackager {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("make_config.yaml");
        std::fs::write(&path, content).unwrap();
        MacOSPkgPackager::from_yaml_file(&path).unwrap()
    }

    #[test]
    fn app_store_installer_identities_are_recognised() {
        assert!(is_app_store_installer_identity(
            "3rd Party Mac Developer Installer: Jane Doe (TEAMID)"
        ));
        assert!(is_app_store_installer_identity(
            "3rd Party Mac Developer Installer"
        ));
        assert!(is_app_store_installer_identity(
            " Mac Installer Distribution: Jane Doe (TEAMID)"
        ));
        assert!(!is_app_store_installer_identity(
            "Developer ID Installer: Jane Doe (TEAMID)"
        ));
        assert!(!is_app_store_installer_identity(
            "3rd Party Mac Developer Application: Jane Doe (TEAMID)"
        ));
        assert!(!is_app_store_installer_identity(""));
    }

    #[test]
    fn make_config_reads_app_store() {
        assert!(
            from_yaml("sign-identity: \"Developer ID Installer: Foo (TEAM)\"\napp-store: true\n")
                .app_store
        );
        assert!(!from_yaml("install-path: /Applications/\n").app_store);
        assert!(!from_yaml("sign-identity: \"Developer ID Installer: Foo (TEAM)\"\n").app_store);
        assert!(!from_yaml("app-store: false\n").app_store);
    }

    #[test]
    fn app_store_mode_is_detected_from_the_installer_identity() {
        let packager = from_yaml(
            "install-path: /Applications\nsign-identity: \"3rd Party Mac Developer Installer\"\n",
        );
        assert!(packager.app_store);
        assert_eq!(packager.install_path.as_deref(), Some("/Applications"));
        assert!(from_yaml("sign-identity: \"Mac Installer Distribution: Foo (TEAM)\"\n").app_store);
        // An explicit `app-store: false` wins over the identity.
        assert!(!from_yaml("sign-identity: \"3rd Party Mac Developer Installer: Foo (TEAM)\"\napp-store: false\n").app_store);
    }

    #[test]
    fn missing_make_config_is_not_app_store() {
        let dir = tempfile::tempdir().unwrap();
        let packager =
            MacOSPkgPackager::from_yaml_file(&dir.path().join("make_config.yaml")).unwrap();
        assert!(!packager.app_store);
        assert!(packager.sign_identity.is_none());
    }

    #[test]
    fn app_store_productbuild_signs_in_one_step() {
        let args = app_store_productbuild_args(
            Path::new("build/Demo.app"),
            Some("/Applications/"),
            "3rd Party Mac Developer Installer: Foo (TEAM)",
            Path::new("dist/1.0.0+1/demo-1.0.0+1-macos.pkg"),
        );
        assert_eq!(
            strings(&args),
            vec![
                "productbuild",
                "--component",
                "build/Demo.app",
                "/Applications",
                "--sign",
                "3rd Party Mac Developer Installer: Foo (TEAM)",
                "dist/1.0.0+1/demo-1.0.0+1-macos.pkg",
            ]
        );
        assert!(
            !strings(&args)
                .iter()
                .any(|a| a == "--scripts" || a == "--timestamp")
        );
        // The install path is passed as Xcode does, without a trailing slash.
        let install_path = |path: Option<&str>| {
            strings(&app_store_productbuild_args(
                Path::new("build/Demo.app"),
                path,
                "Mac Installer Distribution: Foo (TEAM)",
                Path::new("demo.pkg"),
            ))[3]
                .clone()
        };
        assert_eq!(install_path(None), "/Applications");
        assert_eq!(install_path(Some("/Applications")), "/Applications");
        assert_eq!(install_path(Some("/Library/Foo//")), "/Library/Foo");
        assert_eq!(install_path(Some("/")), "/");
    }

    #[test]
    fn direct_download_productbuild_has_no_sign_argument() {
        let args = productbuild_args(
            Path::new("build/Demo.app"),
            None,
            Path::new("dist/demo-unsigned.pkg"),
            None,
        );
        assert_eq!(
            strings(&args),
            vec![
                "productbuild",
                "--component",
                "build/Demo.app",
                "/Applications/",
                "dist/demo-unsigned.pkg",
            ]
        );
    }

    #[test]
    fn app_store_package_needs_an_installer_identity() {
        let packager = MacOSPkgPackager {
            app_store: true,
            ..Default::default()
        };
        let err = packager.package(&test_config()).unwrap_err().to_string();
        assert!(err.contains("`sign-identity`"), "{err}");
        assert!(err.contains("3rd Party Mac Developer Installer"), "{err}");
    }

    #[test]
    fn app_store_package_rejects_scripts() {
        let packager = MacOSPkgPackager {
            app_store: true,
            sign_identity: Some("3rd Party Mac Developer Installer: Foo (TEAM)".into()),
            scripts: Some("macos/packaging/pkg/scripts".into()),
            ..Default::default()
        };
        let err = packager.package(&test_config()).unwrap_err().to_string();
        assert!(err.contains("`scripts`"), "{err}");
        assert!(err.contains("`app-store: true`"), "{err}");
    }

    #[test]
    fn app_store_package_rejects_notarize() {
        let packager = MacOSPkgPackager {
            app_store: true,
            sign_identity: Some("Mac Installer Distribution: Foo (TEAM)".into()),
            notarize: true,
            ..Default::default()
        };
        let err = packager.package(&test_config()).unwrap_err().to_string();
        assert!(err.contains("`notarize`"), "{err}");
        assert!(err.contains("`app-store: true`"), "{err}");

        // `fastforge package --notarize` sets the packaging variable instead.
        let packager = MacOSPkgPackager {
            notarize: false,
            ..packager
        };
        let mut config = test_config();
        config
            .environment
            .insert(notarize::NOTARIZE_ENV.into(), "true".into());
        let err = packager.package(&config).unwrap_err().to_string();
        assert!(err.contains("`--notarize`"), "{err}");
    }

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
    fn notarized_package_is_signed_with_a_timestamp() {
        let args = |timestamp| {
            productsign_args(
                "Developer ID Installer: Foo (TEAM)",
                timestamp,
                Path::new("demo-unsigned.pkg"),
                Path::new("demo.pkg"),
            )
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
        };
        assert_eq!(
            args(true),
            [
                "productsign",
                "--sign",
                "Developer ID Installer: Foo (TEAM)",
                "--timestamp",
                "demo-unsigned.pkg",
                "demo.pkg"
            ]
        );
        assert!(!args(false).contains(&"--timestamp".to_string()));
    }

    #[test]
    fn make_config_reads_notarize() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("make_config.yaml");
        std::fs::write(
            &path,
            "sign-identity: \"Developer ID Installer: Foo (TEAM)\"\nnotarize: true\n",
        )
        .unwrap();
        let packager = MacOSPkgPackager::from_yaml_file(&path).unwrap();
        assert!(packager.notarize);
        std::fs::write(&path, "install-path: /Applications/\n").unwrap();
        assert!(!MacOSPkgPackager::from_yaml_file(&path).unwrap().notarize);
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
            "<installer-gui-script>\n    <domains enable_localSystem=\"true\" enable_currentUserHome=\"false\" enable_anywhere=\"false\" />\n    <options customize=\"never\"/>\n    <options x=\"y\"/>\n</installer-gui-script>"
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
