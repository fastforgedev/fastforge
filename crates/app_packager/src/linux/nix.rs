use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

use super::common::{FormatVariables, RawPackaging, render_desktop_entry, var};
use super::staging::{self, Contents, Layout};

/// Packages a Flutter Linux app for Nix: a directory holding the staged
/// application (`usr/`) and a `default.nix` that installs it, patches its
/// ELF files against nixpkgs' libraries with `autoPatchelfHook`, and wraps
/// the binary with the GTK environment. Build it with `nix-build <dir>` or
/// install it with `nix profile install --file <dir>`.
///
/// A `default.nix` in `.fastforge/packaging/linux/nix/` replaces the
/// generated one. Needs no tools at packaging time.
pub struct LinuxNixPackager;

/// The Nix package name: letters, digits and `-` (the binary name with `_`
/// turned into `-`).
fn pname(binary_name: &str) -> String {
    binary_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Escapes a value for a Nix double-quoted string.
fn nix_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace("${", "\\${")
    )
}

/// The generated derivation. The staged `usr/` tree becomes `$out`, the
/// binary gets a wrapper in `$out/bin`, and the libraries a Flutter
/// desktop app and its plugins usually link against are available to
/// `autoPatchelfHook`; `extraBuildInputs` adds more.
fn default_nix(
    pname: &str,
    version: &str,
    binary: &str,
    description: Option<&str>,
    homepage: Option<&str>,
    license: Option<&str>,
) -> String {
    let mut meta = vec![format!("    mainProgram = {};", nix_string(binary))];
    if let Some(description) = description {
        meta.push(format!("    description = {};", nix_string(description)));
    }
    if let Some(homepage) = homepage {
        meta.push(format!("    homepage = {};", nix_string(homepage)));
    }
    if let Some(license) = license {
        meta.push(format!(
            "    license = pkgs.lib.getLicenseFromSpdxId {};",
            nix_string(license)
        ));
    }
    meta.push("    platforms = pkgs.lib.platforms.linux;".to_string());
    format!(
        r#"{{ pkgs ? import <nixpkgs> {{ }}, extraBuildInputs ? [ ] }}:

pkgs.stdenv.mkDerivation {{
  pname = {pname};
  version = {version};
  src = ./usr;

  nativeBuildInputs = with pkgs; [ autoPatchelfHook makeWrapper wrapGAppsHook3 ];
  buildInputs = with pkgs; [
    gtk3
    glib
    pango
    cairo
    harfbuzz
    atk
    gdk-pixbuf
    libepoxy
    fontconfig
    zlib
    stdenv.cc.cc.lib
  ] ++ extraBuildInputs;

  dontConfigure = true;
  dontBuild = true;
  dontWrapGApps = true;
  dontStrip = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -a . $out/
    runHook postInstall
  '';

  # The engine and plugins in lib/ are found through the binary's RPATH.
  preFixup = ''
    addAutoPatchelfSearchPath $out/lib/{binary_path}/lib
  '';

  postFixup = ''
    mkdir -p $out/bin
    makeWrapper $out/lib/{binary_path}/{binary_path} $out/bin/{binary_path} "''${{gappsWrapperArgs[@]}}"
  '';

  meta = {{
{meta}
  }};
}}
"#,
        pname = nix_string(pname),
        version = nix_string(version),
        binary_path = binary,
        meta = meta.join("\n"),
    )
}

impl AppPackager for LinuxNixPackager {
    fn name(&self) -> &str {
        "nix"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn package_format(&self) -> &str {
        "nix"
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        // The artifact is a directory, `<artifact name>.nix/`.
        let output_dir = config.output_file()?;
        if output_dir.exists() {
            std::fs::remove_dir_all(&output_dir)?;
        }
        std::fs::create_dir_all(&output_dir)?;
        let binary_name = &config.app_binary_name;
        let version = config.app_version.replace('+', "-");
        let raw = RawPackaging::load(
            config,
            "nix",
            FormatVariables {
                package_name: None,
                default_package_name: pname(binary_name),
                package_version: version.clone(),
                package_arch: super::common::machine_architecture().to_string(),
                install_dir: Some(format!("$out/lib/{}", binary_name)),
                display_name: None,
                packaging_dir: &output_dir,
                output_file: &output_dir,
                extra: vec![],
            },
        )?;
        staging::stage(
            &raw,
            config,
            &output_dir,
            &Layout::flatpak(binary_name),
            Contents {
                icon: raw.icon(None),
                metainfo: None,
            },
            || {
                let variables = raw.variables();
                render_desktop_entry(&[
                    ("Type", Some("Application".to_string())),
                    (
                        "Name",
                        var(variables, "APP_DISPLAY_NAME")
                            .or_else(|| Some(config.app_name.clone())),
                    ),
                    ("Comment", var(variables, "APP_DESCRIPTION")),
                    ("Exec", Some(format!("{} %U", binary_name))),
                    ("Icon", Some(raw.app_id().to_string())),
                    ("Categories", Some("Utility;".to_string())),
                ])
            },
        )?;

        let variables = raw.variables();
        let default = match raw.file(&["default.nix"]) {
            Some(src) => raw.render(&src)?,
            None => default_nix(
                &var(variables, "PACKAGE_NAME").unwrap_or_else(|| pname(binary_name)),
                &version,
                binary_name,
                var(variables, "APP_DESCRIPTION").as_deref(),
                var(variables, "APP_HOMEPAGE").as_deref(),
                var(variables, "APP_LICENSE").as_deref(),
            ),
        };
        std::fs::write(output_dir.join("default.nix"), default)?;
        let mut result = PackageResult::resolve(output_dir, true)?;
        result.config = Some(config.clone());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_strings_are_nix_safe() {
        assert_eq!(pname("hello_world"), "hello-world");
        assert_eq!(nix_string("a \"b\" ${c}"), r#""a \"b\" \${c}""#);
    }

    #[test]
    fn derivation_wraps_the_bundled_binary() {
        let nix = default_nix(
            "hello-world",
            "1.2.3-4",
            "hello_world",
            Some("A demo"),
            None,
            Some("MIT"),
        );
        assert!(nix.contains("pname = \"hello-world\";\n  version = \"1.2.3-4\";\n  src = ./usr;"));
        assert!(nix.contains("addAutoPatchelfSearchPath $out/lib/hello_world/lib"));
        assert!(nix.contains(
            "makeWrapper $out/lib/hello_world/hello_world $out/bin/hello_world \"''${gappsWrapperArgs[@]}\""
        ));
        assert!(nix.contains("license = pkgs.lib.getLicenseFromSpdxId \"MIT\";"));
        assert!(nix.contains("mainProgram = \"hello_world\";"));
    }

    /// Packages the bundle at `FASTFORGE_NIX_BUNDLE` (binary `nix_smoke`)
    /// into `FASTFORGE_NIX_OUT`, for building with nix-build.
    #[test]
    #[ignore = "packages a prebuilt bundle for a nix-build check"]
    fn packages_a_bundle_for_nix_build() {
        let bundle = std::path::PathBuf::from(std::env::var("FASTFORGE_NIX_BUNDLE").unwrap());
        let out = std::path::PathBuf::from(std::env::var("FASTFORGE_NIX_OUT").unwrap());
        let config = PackageConfig {
            app_name: "nix_smoke".into(),
            app_binary_name: "nix_smoke".into(),
            app_version: "1.0.0+1".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "nix".into(),
            is_installer: false,
            build_output_dir: bundle,
            build_output_files: vec![],
            output_dir: out,
            packaging_root: None,
            environment: Default::default(),
        };
        println!(
            "{}",
            LinuxNixPackager.package(&config).unwrap().artifacts[0].display()
        );
    }

    #[test]
    fn packages_a_directory_with_the_staged_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("bundle");
        std::fs::create_dir_all(bundle.join("lib")).unwrap();
        std::fs::write(bundle.join("hello_world"), "bin").unwrap();
        std::fs::write(bundle.join("lib/libapp.so"), "so").unwrap();
        let config = PackageConfig {
            app_name: "hello_world".into(),
            app_binary_name: "hello_world".into(),
            app_version: "1.2.3+4".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "nix".into(),
            is_installer: false,
            build_output_dir: bundle,
            build_output_files: vec![],
            output_dir: tmp.path().join("dist"),
            packaging_root: None,
            environment: Default::default(),
        };
        let result = LinuxNixPackager.package(&config).unwrap();
        let dir = &result.artifacts[0];
        assert!(dir.ends_with("dist/1.2.3+4/hello_world-1.2.3+4-linux.nix"));
        assert!(dir.join("default.nix").is_file());
        assert!(dir.join("usr/lib/hello_world/lib/libapp.so").is_file());
        assert!(
            dir.join("usr/share/applications")
                .read_dir()
                .unwrap()
                .next()
                .is_some()
        );
    }
}
