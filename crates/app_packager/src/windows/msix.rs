use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

/// Builds a Windows `.msix` package using the `makeappx` / `signtool` SDK tools,
/// mirroring Dart's `AppPackageMakerMsix` (which delegates to the `msix` pub
/// package).
///
/// Reads `windows/packaging/msix/make_config.yaml` when present (same schema
/// as Dart's `MakeMsixConfig`); falls back to sensible defaults otherwise.
///
/// Requires the Windows 10 SDK tools (`makeappx`, `signtool`) on Windows.
#[derive(Default)]
pub struct WindowsMsixPackager {
    /// Optional path to a PFX certificate for signing (overrides make_config).
    pub certificate_path: Option<String>,
    /// Optional PFX certificate password (overrides make_config).
    pub certificate_password: Option<String>,
    /// Publisher distinguished name used in AppxManifest (overrides
    /// make_config), e.g. `"CN=My Company, O=My Company, C=US"`.
    pub publisher: Option<String>,
}

/// Schema of `windows/packaging/msix/make_config.yaml`, mirroring Dart's
/// `MakeMsixConfig`. Values may be written as strings or natural YAML types.
#[derive(Debug, Default, Deserialize)]
pub struct MsixMakeConfig {
    pub display_name: Option<String>,
    pub publisher_display_name: Option<String>,
    pub identity_name: Option<String>,
    pub msix_version: Option<String>,
    pub logo_path: Option<String>,
    /// Comma-separated capability list, e.g. `"internetClient,microphone"`.
    pub capabilities: Option<String>,
    /// Comma-separated language list, e.g. `"en-us, zh-cn"`.
    pub languages: Option<String>,
    /// Comma-separated file extensions the app registers to open.
    pub file_extension: Option<String>,
    /// Protocol activation scheme(s), comma-separated.
    pub protocol_activation: Option<String>,
    pub add_execution_alias: Option<String>,
    pub enable_at_startup: Option<String>,
    pub architecture: Option<String>,
    pub certificate_path: Option<String>,
    pub certificate_password: Option<String>,
    pub publisher: Option<String>,
    /// Extra options passed verbatim to `signtool sign`.
    pub signtool_options: Option<String>,
    /// If `"false"`, don't sign the msix file.
    pub sign_msix: Option<String>,
}

impl MsixMakeConfig {
    fn load() -> Result<Self, PackageError> {
        let path = Path::new("windows/packaging/msix/make_config.yaml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            PackageError::General(format!("Failed to read {}: {}", path.display(), e))
        })?;
        serde_yaml::from_str(&content).map_err(|e| {
            PackageError::General(format!("Failed to parse {}: {}", path.display(), e))
        })
    }
}

fn is_false(value: &Option<String>) -> bool {
    value
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("false"))
}

fn is_true(value: &Option<String>) -> bool {
    value
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

fn split_list(value: &Option<String>) -> Vec<String> {
    value
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Derives an `a.b.c.d` MSIX version from the app version (e.g. `1.2.3+4`
/// becomes `1.2.3.0`), mirroring the msix pub package's default.
fn msix_version_from(app_version: &str) -> String {
    let base = app_version.split('+').next().unwrap_or(app_version);
    let numeric: Vec<String> = base
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
        })
        .map(|p| if p.is_empty() { "0".to_string() } else { p })
        .collect();
    let mut parts = numeric;
    parts.resize(3, "0".to_string());
    format!("{}.{}.{}.0", parts[0], parts[1], parts[2])
}

/// Detects the target architecture from the build output directory path
/// (mirrors Dart's `_detectArchitecture`): Flutter 3.22+ uses
/// `build/windows/{x64,arm64}/runner/Release`.
fn detect_architecture(build_output_dir: &Path) -> &'static str {
    if build_output_dir
        .to_string_lossy()
        .to_lowercase()
        .contains("arm64")
    {
        "arm64"
    } else {
        "x64"
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Renders the AppxManifest.xml from the configuration.
fn render_manifest(
    make_config: &MsixMakeConfig,
    config: &PackageConfig,
    publisher: &str,
    logo_file: Option<&str>,
) -> String {
    let display_name = make_config
        .display_name
        .clone()
        .unwrap_or_else(|| config.app_name.clone());
    let identity_name = make_config
        .identity_name
        .clone()
        .unwrap_or_else(|| config.app_name.clone());
    let version = make_config
        .msix_version
        .clone()
        .unwrap_or_else(|| msix_version_from(&config.app_version));
    let publisher_display_name = make_config
        .publisher_display_name
        .clone()
        .unwrap_or_else(|| "Publisher".to_string());
    let architecture = make_config
        .architecture
        .clone()
        .unwrap_or_else(|| detect_architecture(&config.build_output_dir).to_string());

    let logo = logo_file.unwrap_or("Assets\\StoreLogo.png");

    let languages = {
        let list = split_list(&make_config.languages);
        let list = if list.is_empty() {
            vec!["en-us".to_string()]
        } else {
            list
        };
        list.into_iter()
            .map(|l| format!("    <Resource Language=\"{}\"/>", xml_escape(&l)))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut capabilities: Vec<String> =
        vec!["    <rescap:Capability Name=\"runFullTrust\"/>".to_string()];
    for capability in split_list(&make_config.capabilities) {
        // Device capabilities vs regular capabilities (subset of the msix
        // pub package's mapping; unknown names fall back to <Capability>).
        let device_caps = ["microphone", "webcam", "location", "bluetooth"];
        if device_caps.contains(&capability.as_str()) {
            capabilities.push(format!(
                "    <DeviceCapability Name=\"{}\"/>",
                xml_escape(&capability)
            ));
        } else {
            capabilities.push(format!(
                "    <Capability Name=\"{}\"/>",
                xml_escape(&capability)
            ));
        }
    }

    // Application-level extensions
    let mut extensions = Vec::new();
    let file_extensions = split_list(&make_config.file_extension);
    if !file_extensions.is_empty() {
        let types = file_extensions
            .iter()
            .map(|e| {
                let ext = e.strip_prefix('.').unwrap_or(e);
                format!("              <uap:FileType>.{}</uap:FileType>", xml_escape(ext))
            })
            .collect::<Vec<_>>()
            .join("\n");
        extensions.push(format!(
            "        <uap:Extension Category=\"windows.fileTypeAssociation\">\n          <uap:FileTypeAssociation Name=\"fileassociations\">\n            <uap:SupportedFileTypes>\n{}\n            </uap:SupportedFileTypes>\n          </uap:FileTypeAssociation>\n        </uap:Extension>",
            types
        ));
    }
    for protocol in split_list(&make_config.protocol_activation) {
        extensions.push(format!(
            "        <uap:Extension Category=\"windows.protocol\">\n          <uap:Protocol Name=\"{}\"/>\n        </uap:Extension>",
            xml_escape(&protocol)
        ));
    }
    if let Some(alias) = make_config
        .add_execution_alias
        .as_deref()
        .filter(|a| !a.is_empty() && !a.eq_ignore_ascii_case("false"))
    {
        let alias_name = if alias.eq_ignore_ascii_case("true") {
            config.app_name.replace('_', "").to_lowercase()
        } else {
            alias.to_string()
        };
        extensions.push(format!(
            "        <uap3:Extension Category=\"windows.appExecutionAlias\" Executable=\"{bin}.exe\" EntryPoint=\"Windows.FullTrustApplication\">\n          <uap3:AppExecutionAlias>\n            <desktop:ExecutionAlias Alias=\"{alias}.exe\"/>\n          </uap3:AppExecutionAlias>\n        </uap3:Extension>",
            bin = xml_escape(&config.app_binary_name),
            alias = xml_escape(&alias_name),
        ));
    }
    if is_true(&make_config.enable_at_startup) {
        extensions.push(format!(
            "        <desktop:Extension Category=\"windows.startupTask\" Executable=\"{bin}.exe\" EntryPoint=\"Windows.FullTrustApplication\">\n          <desktop:StartupTask TaskId=\"startupTask\" Enabled=\"true\" DisplayName=\"{name}\"/>\n        </desktop:Extension>",
            bin = xml_escape(&config.app_binary_name),
            name = xml_escape(&display_name),
        ));
    }
    let extensions_block = if extensions.is_empty() {
        String::new()
    } else {
        format!("\n      <Extensions>\n{}\n      </Extensions>", extensions.join("\n"))
    };

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
         xmlns:uap3="http://schemas.microsoft.com/appx/manifest/uap/windows10/3"
         xmlns:desktop="http://schemas.microsoft.com/appx/manifest/desktop/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities">
  <Identity Name="{identity}" Publisher="{publisher}" Version="{version}" ProcessorArchitecture="{arch}"/>
  <Properties>
    <DisplayName>{display_name}</DisplayName>
    <PublisherDisplayName>{publisher_display_name}</PublisherDisplayName>
    <Logo>{logo}</Logo>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Universal" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22000.0"/>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22000.0"/>
  </Dependencies>
  <Resources>
{languages}
  </Resources>
  <Applications>
    <Application Id="App" Executable="{bin}.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="{display_name}" Description="{display_name}" BackgroundColor="transparent" Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png">
        <uap:DefaultTile/>
        <uap:SplashScreen Image="Assets\SplashScreen.png"/>
      </uap:VisualElements>{extensions}
    </Application>
  </Applications>
  <Capabilities>
{capabilities}
  </Capabilities>
</Package>
"#,
        identity = xml_escape(&identity_name),
        publisher = xml_escape(publisher),
        version = xml_escape(&version),
        arch = xml_escape(&architecture),
        display_name = xml_escape(&display_name),
        publisher_display_name = xml_escape(&publisher_display_name),
        logo = logo,
        languages = languages,
        bin = xml_escape(&config.app_binary_name),
        extensions = extensions_block,
        capabilities = capabilities.join("\n"),
    )
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

impl AppPackager for WindowsMsixPackager {
    fn name(&self) -> &str {
        "msix"
    }

    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn package_format(&self) -> &str {
        "msix"
    }

    #[cfg(not(target_os = "windows"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = MsixMakeConfig::load()?;
        let pkg_dir = config.packaging_dir();

        // Copy the flutter build output
        run(Command::new("xcopy").args([
            "/E",
            "/I",
            "/Q",
            &config.build_output_dir.display().to_string(),
            &pkg_dir.display().to_string(),
        ]))?;

        // Install the configured logo into Assets/ (used for all logo slots)
        let logo_file = if let Some(logo_path) = &make_config.logo_path {
            let logo = Path::new(logo_path);
            if !logo.exists() {
                return Err(PackageError::NotFound(format!(
                    "logo_path {} doesn't exist",
                    logo_path
                )));
            }
            let assets_dir = pkg_dir.join("Assets");
            std::fs::create_dir_all(&assets_dir)?;
            for name in [
                "StoreLogo.png",
                "Square150x150Logo.png",
                "Square44x44Logo.png",
                "SplashScreen.png",
            ] {
                std::fs::copy(logo, assets_dir.join(name))?;
            }
            Some("Assets\\StoreLogo.png")
        } else {
            None
        };

        // Publisher: struct field overrides make_config, then default
        let publisher = self
            .publisher
            .clone()
            .or_else(|| make_config.publisher.clone())
            .unwrap_or_else(|| "CN=Publisher".to_string());

        let manifest = render_manifest(&make_config, config, &publisher, logo_file);
        std::fs::write(pkg_dir.join("AppxManifest.xml"), &manifest)?;

        let output_file = config.output_file();
        run(Command::new("makeappx").args([
            "pack",
            "/d",
            &pkg_dir.display().to_string(),
            "/p",
            &output_file.display().to_string(),
            "/nv",
            "/o",
        ]))?;

        // Sign unless disabled (mirrors the msix pub package's sign_msix flag)
        let certificate_path = self
            .certificate_path
            .clone()
            .or_else(|| make_config.certificate_path.clone());
        let certificate_password = self
            .certificate_password
            .clone()
            .or_else(|| make_config.certificate_password.clone());

        if !is_false(&make_config.sign_msix) {
            if let Some(signtool_options) = &make_config.signtool_options {
                let mut args: Vec<String> = vec!["sign".to_string()];
                args.extend(
                    signtool_options
                        .split_whitespace()
                        .map(String::from),
                );
                args.push(output_file.display().to_string());
                run(Command::new("signtool").args(&args))?;
            } else if let Some(cert) = &certificate_path {
                let mut args = vec![
                    "sign".to_string(),
                    "/fd".to_string(),
                    "SHA256".to_string(),
                    "/a".to_string(),
                    "/f".to_string(),
                    cert.clone(),
                ];
                if let Some(pwd) = &certificate_password {
                    args.push("/p".to_string());
                    args.push(pwd.clone());
                }
                args.push(output_file.display().to_string());
                run(Command::new("signtool").args(&args))?;
            }
        }

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
            platform: Platform::Windows,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "msix".into(),
            is_installer: false,
            build_output_dir: PathBuf::from("build/windows/x64/runner/Release"),
            build_output_files: vec![],
            output_dir: PathBuf::new(),
        }
    }

    #[test]
    fn msix_version_derivation() {
        assert_eq!(msix_version_from("1.2.3+4"), "1.2.3.0");
        assert_eq!(msix_version_from("1.2.3"), "1.2.3.0");
        assert_eq!(msix_version_from("2.0.0-beta.1+7"), "2.0.0.0");
        assert_eq!(msix_version_from("1.2"), "1.2.0.0");
    }

    #[test]
    fn architecture_detection() {
        assert_eq!(
            detect_architecture(Path::new("build/windows/arm64/runner/Release")),
            "arm64"
        );
        assert_eq!(
            detect_architecture(Path::new("build/windows/x64/runner/Release")),
            "x64"
        );
        assert_eq!(detect_architecture(Path::new("build/windows/runner")), "x64");
    }

    #[test]
    fn manifest_with_full_config() {
        let mc: MsixMakeConfig = serde_yaml::from_str(
            r#"
display_name: Hola Amigos
publisher_display_name: ACME Corp
identity_name: com.acme.hola
msix_version: 2.0.1.0
languages: "en-us, zh-cn"
capabilities: "internetClient,microphone"
file_extension: ".txt,.md"
protocol_activation: holaamigos
add_execution_alias: hola
enable_at_startup: "true"
architecture: arm64
"#,
        )
        .unwrap();
        let manifest = render_manifest(&mc, &test_config(), "CN=ACME", None);
        assert!(manifest.contains(
            r#"<Identity Name="com.acme.hola" Publisher="CN=ACME" Version="2.0.1.0" ProcessorArchitecture="arm64"/>"#
        ));
        assert!(manifest.contains("<DisplayName>Hola Amigos</DisplayName>"));
        assert!(manifest.contains("<PublisherDisplayName>ACME Corp</PublisherDisplayName>"));
        assert!(manifest.contains(r#"<Resource Language="en-us"/>"#));
        assert!(manifest.contains(r#"<Resource Language="zh-cn"/>"#));
        assert!(manifest.contains(r#"<Capability Name="internetClient"/>"#));
        assert!(manifest.contains(r#"<DeviceCapability Name="microphone"/>"#));
        assert!(manifest.contains(r#"<rescap:Capability Name="runFullTrust"/>"#));
        assert!(manifest.contains("<uap:FileType>.txt</uap:FileType>"));
        assert!(manifest.contains("<uap:FileType>.md</uap:FileType>"));
        assert!(manifest.contains(r#"<uap:Protocol Name="holaamigos"/>"#));
        assert!(manifest.contains(r#"<desktop:ExecutionAlias Alias="hola.exe"/>"#));
        assert!(manifest.contains("windows.startupTask"));
    }

    #[test]
    fn manifest_defaults() {
        let manifest = render_manifest(
            &MsixMakeConfig::default(),
            &test_config(),
            "CN=Publisher",
            None,
        );
        assert!(manifest.contains(
            r#"<Identity Name="hola_amigos" Publisher="CN=Publisher" Version="1.2.3.0" ProcessorArchitecture="x64"/>"#
        ));
        assert!(manifest.contains(r#"<Resource Language="en-us"/>"#));
        assert!(!manifest.contains("<Extensions>"));
    }
}
