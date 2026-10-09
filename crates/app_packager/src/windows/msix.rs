use std::io::Write;
use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::{Deserialize, Deserializer};

use super::common::{RawPackaging, architecture, find_executable};

/// Builds MSIX packages with the Windows SDK, from a staged application.
/// Raw AppxManifest.xml and file overlays live in `.fastforge/packaging/windows`.
/// Legacy make_config.yaml and pubspec msix_config remain readable, regardless
/// of whether the Flutter project depends on the Dart msix package.
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

/// Accepts strings as well as natural YAML scalars (`true`, `1`) for the
/// string-typed msix options.
fn opt_scalar<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    let value: Option<serde_yaml::Value> = Option::deserialize(deserializer)?;
    Ok(value.and_then(|value| match value {
        serde_yaml::Value::Null => None,
        serde_yaml::Value::String(s) => Some(s),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        other => serde_yaml::to_string(&other)
            .ok()
            .map(|s| s.trim().to_string()),
    }))
}

macro_rules! msix_make_config {
    ($($(#[$doc:meta])* $field:ident),* $(,)?) => {
        /// Schema of `windows/packaging/msix/make_config.yaml` (and of
        /// `pubspec.yaml`'s `msix_config:`), mirroring Dart's
        /// `MakeMsixConfig`. Values may be written as strings or natural YAML
        /// scalars.
        #[derive(Debug, Default, Clone, Deserialize)]
        pub struct MsixMakeConfig {
            $(
                $(#[$doc])*
                #[serde(default, deserialize_with = "opt_scalar")]
                pub $field: Option<String>,
            )*
        }

        impl MsixMakeConfig {
            /// Field-wise merge: values in `self` win, `fallback` fills the
            /// gaps (make_config overrides `pubspec.yaml`'s `msix_config`).
            pub fn or(self, fallback: MsixMakeConfig) -> MsixMakeConfig {
                MsixMakeConfig {
                    $($field: self.$field.or(fallback.$field),)*
                }
            }
        }
    };
}

msix_make_config! {
    display_name,
    publisher_display_name,
    identity_name,
    msix_version,
    logo_path,
    /// Legacy Dart option, accepted but not applied by the native packager.
    trim_logo,
    /// Comma-separated capability list, e.g. `"internetClient,microphone"`.
    capabilities,
    /// Comma-separated language list, e.g. `"en-us, zh-cn"`.
    languages,
    /// Comma-separated file extensions the app registers to open.
    file_extension,
    /// Protocol activation scheme(s), comma-separated.
    protocol_activation,
    /// Dart's key: `true` derives the alias from the app name, any other
    /// value except `false` is used as the alias.
    add_execution_alias,
    /// The msix package's own key for the execution alias.
    execution_alias,
    enable_at_startup,
    store,
    debug,
    output_path,
    output_name,
    architecture,
    build_windows,
    certificate_path,
    certificate_password,
    publisher,
    /// Extra options passed verbatim to `signtool sign`.
    signtool_options,
    /// If `"false"`, don't sign the msix file.
    sign_msix,
    install_certificate,
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
        serde_yaml::from_str::<Option<Self>>(&content)
            .map(Option::unwrap_or_default)
            .map_err(|e| {
                PackageError::General(format!("Failed to parse {}: {}", path.display(), e))
            })
    }

    /// The execution alias, from `execution_alias` or Dart's
    /// `add_execution_alias` (`true` → app name without underscores,
    /// lowercased).
    fn resolved_execution_alias(&self, app_name: &str) -> Option<String> {
        if let Some(alias) = self.execution_alias.as_deref().filter(|a| !a.is_empty()) {
            return Some(alias.to_string());
        }
        let alias = self
            .add_execution_alias
            .as_deref()
            .filter(|a| !a.is_empty() && !a.eq_ignore_ascii_case("false"))?;
        if alias.eq_ignore_ascii_case("true") {
            Some(app_name.replace('_', "").to_lowercase())
        } else {
            Some(alias.to_string())
        }
    }
}

/// The parts of `pubspec.yaml` the msix packager cares about.
#[derive(Debug, Default)]
struct PubspecMsix {
    msix_config: MsixMakeConfig,
}

impl PubspecMsix {
    fn load() -> Self {
        std::fs::read_to_string("pubspec.yaml")
            .ok()
            .and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok())
            .map(|value| Self::from_value(&value))
            .unwrap_or_default()
    }

    fn from_value(pubspec: &serde_yaml::Value) -> Self {
        Self {
            msix_config: pubspec
                .get("msix_config")
                .cloned()
                .and_then(|v| serde_yaml::from_value(v).ok())
                .unwrap_or_default(),
        }
    }
}

fn is_false(value: &Option<String>) -> bool {
    value
        .as_deref()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("false"))
}

fn is_true(value: &Option<String>) -> bool {
    value
        .as_deref()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
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
    architecture(build_output_dir)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Fully resolved manifest values for the native packager, with the msix
/// pub package's defaults applied.
#[derive(Debug, Clone, PartialEq)]
struct MsixManifest {
    identity_name: String,
    publisher: String,
    version: String,
    architecture: String,
    display_name: String,
    publisher_display_name: String,
    description: String,
    executable: String,
    languages: Vec<String>,
    capabilities: Vec<String>,
    file_extensions: Vec<String>,
    protocols: Vec<String>,
    execution_alias: Option<String>,
    enable_at_startup: bool,
}

/// Assets written into the package and referenced by the manifest.
const LOGO_ASSETS: [&str; 4] = [
    "StoreLogo.png",
    "Square150x150Logo.png",
    "Square44x44Logo.png",
    "SplashScreen.png",
];

impl MsixManifest {
    /// Applies the msix package's defaults (`validateConfigValues`):
    /// display name = app name without `_`, identity =
    /// `com.flutter.<name>`, publisher display name = identity, version from
    /// pubspec (`a.b.c.0`), description from pubspec, `en-us` language.
    fn resolve(
        mc: &MsixMakeConfig,
        config: &PackageConfig,
        publisher: String,
        description: Option<String>,
        executable: String,
    ) -> Self {
        let clean_app_name = config.app_name.replace('_', "");
        let identity_name = mc
            .identity_name
            .clone()
            .unwrap_or_else(|| format!("com.flutter.{}", clean_app_name));
        let languages = {
            let list = split_list(&mc.languages);
            if list.is_empty() {
                vec!["en-us".to_string()]
            } else {
                list
            }
        };
        let protocols = split_list(&mc.protocol_activation)
            .into_iter()
            .map(|p| {
                p.to_lowercase()
                    .replace("://", "")
                    .replace(":/", "")
                    .replace(':', "")
            })
            .filter(|p| !p.is_empty())
            .collect();
        Self {
            publisher_display_name: mc
                .publisher_display_name
                .clone()
                .unwrap_or_else(|| identity_name.clone()),
            identity_name,
            publisher,
            version: mc
                .msix_version
                .clone()
                .unwrap_or_else(|| msix_version_from(&config.app_version)),
            architecture: mc
                .architecture
                .clone()
                .unwrap_or_else(|| detect_architecture(&config.build_output_dir).to_string()),
            display_name: mc.display_name.clone().unwrap_or(clean_app_name),
            description: description.unwrap_or_else(|| config.app_name.clone()),
            executable,
            languages,
            capabilities: split_list(&mc.capabilities),
            file_extensions: split_list(&mc.file_extension)
                .into_iter()
                .map(|e| e.trim_start_matches('.').to_string())
                .collect(),
            protocols,
            execution_alias: mc.resolved_execution_alias(&config.app_name),
            enable_at_startup: is_true(&mc.enable_at_startup),
        }
    }

    /// Renders AppxManifest.xml. Every image it references is one of
    /// [`LOGO_ASSETS`] under `Assets\`.
    fn render(&self) -> String {
        let languages = self
            .languages
            .iter()
            .map(|l| format!("    <Resource Language=\"{}\"/>", xml_escape(l)))
            .collect::<Vec<_>>()
            .join("\n");

        let mut capabilities: Vec<String> =
            vec!["    <rescap:Capability Name=\"runFullTrust\"/>".to_string()];
        for capability in &self.capabilities {
            // Device capabilities vs regular capabilities (subset of the msix
            // pub package's mapping; unknown names fall back to <Capability>).
            let device_caps = ["microphone", "webcam", "location", "bluetooth"];
            if device_caps.contains(&capability.as_str()) {
                capabilities.push(format!(
                    "    <DeviceCapability Name=\"{}\"/>",
                    xml_escape(capability)
                ));
            } else {
                capabilities.push(format!(
                    "    <Capability Name=\"{}\"/>",
                    xml_escape(capability)
                ));
            }
        }

        let exe = xml_escape(&self.executable);
        let mut extensions = Vec::new();
        if !self.file_extensions.is_empty() {
            let types = self
                .file_extensions
                .iter()
                .map(|ext| {
                    format!(
                        "              <uap:FileType>.{}</uap:FileType>",
                        xml_escape(ext)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            extensions.push(format!(
                "        <uap:Extension Category=\"windows.fileTypeAssociation\">\n          <uap:FileTypeAssociation Name=\"fileassociations\">\n            <uap:SupportedFileTypes>\n{}\n            </uap:SupportedFileTypes>\n          </uap:FileTypeAssociation>\n        </uap:Extension>",
                types
            ));
        }
        for protocol in &self.protocols {
            extensions.push(format!(
                "        <uap:Extension Category=\"windows.protocol\">\n          <uap:Protocol Name=\"{}\"/>\n        </uap:Extension>",
                xml_escape(protocol)
            ));
        }
        if let Some(alias) = &self.execution_alias {
            extensions.push(format!(
                "        <uap3:Extension Category=\"windows.appExecutionAlias\" Executable=\"{exe}\" EntryPoint=\"Windows.FullTrustApplication\">\n          <uap3:AppExecutionAlias>\n            <desktop:ExecutionAlias Alias=\"{alias}.exe\"/>\n          </uap3:AppExecutionAlias>\n        </uap3:Extension>",
                alias = xml_escape(alias.trim_end_matches(".exe")),
            ));
        }
        if self.enable_at_startup {
            extensions.push(format!(
                "        <desktop:Extension Category=\"windows.startupTask\" Executable=\"{exe}\" EntryPoint=\"Windows.FullTrustApplication\">\n          <desktop:StartupTask TaskId=\"startupTask\" Enabled=\"true\" DisplayName=\"{name}\"/>\n        </desktop:Extension>",
                name = xml_escape(&self.display_name),
            ));
        }
        let extensions_block = if extensions.is_empty() {
            String::new()
        } else {
            format!(
                "\n      <Extensions>\n{}\n      </Extensions>",
                extensions.join("\n")
            )
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
    <Logo>Assets\StoreLogo.png</Logo>
    <Description>{description}</Description>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Universal" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22000.0"/>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22000.0"/>
  </Dependencies>
  <Resources>
{languages}
  </Resources>
  <Applications>
    <Application Id="App" Executable="{exe}" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="{display_name}" Description="{description}" BackgroundColor="transparent" Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png">
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
            identity = xml_escape(&self.identity_name),
            publisher = xml_escape(&self.publisher),
            version = xml_escape(&self.version),
            arch = xml_escape(&self.architecture),
            display_name = xml_escape(&self.display_name),
            publisher_display_name = xml_escape(&self.publisher_display_name),
            description = xml_escape(&self.description),
            languages = languages,
            exe = exe,
            extensions = extensions_block,
            capabilities = capabilities.join("\n"),
        )
    }
}

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Returns the largest PNG-encoded image embedded in an `.ico` file.
fn png_from_ico(data: &[u8]) -> Option<Vec<u8>> {
    let u16le = |at: usize| Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
    let u32le = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    if u16le(0)? != 0 || u16le(2)? != 1 {
        return None;
    }
    let count = u16le(4)? as usize;
    let mut best: Option<(u32, &[u8])> = None;
    for i in 0..count {
        let entry = 6 + i * 16;
        let width = match *data.get(entry)? {
            0 => 256,
            w => u32::from(w),
        };
        let size = u32le(entry + 8)? as usize;
        let offset = u32le(entry + 12)? as usize;
        let Some(image) = data.get(offset..offset.checked_add(size)?) else {
            continue;
        };
        if image.starts_with(PNG_SIGNATURE) && best.is_none_or(|(w, _)| width > w) {
            best = Some((width, image));
        }
    }
    best.map(|(_, image)| image.to_vec())
}

/// Encodes a solid-color square RGBA PNG (used as a placeholder logo).
fn solid_png(size: u32, rgba: [u8; 4]) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(kind);
        hasher.update(data);
        out.extend_from_slice(&hasher.finalize().to_be_bytes());
    }
    // Each scanline: filter byte 0 (none) followed by `size` RGBA pixels.
    let mut row = vec![0u8];
    row.extend(rgba.repeat(size as usize));
    let raw = row.repeat(size as usize);
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(&raw)
        .expect("writing to an in-memory buffer cannot fail");
    let compressed = encoder
        .finish()
        .expect("writing to an in-memory buffer cannot fail");

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA, no interlace

    let mut png = PNG_SIGNATURE.to_vec();
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &compressed);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// Resolves the logo PNG for the native packager: `logo_path` (must exist),
/// else the PNG embedded in the Flutter runner icon
/// (`windows/runner/resources/app_icon.ico`), else a generated placeholder.
fn resolve_logo(logo_path: Option<&str>, runner_icon: &Path) -> Result<Vec<u8>, PackageError> {
    if let Some(logo_path) = logo_path {
        let logo = Path::new(logo_path);
        if !logo.is_file() {
            return Err(PackageError::NotFound(format!(
                "Logo file not found at {}",
                logo_path
            )));
        }
        return Ok(std::fs::read(logo)?);
    }
    if let Some(png) = std::fs::read(runner_icon).ok().and_then(|data| {
        png_from_ico(&data).or_else(|| image::load_from_memory(&data).ok().map(|_| data))
    }) {
        return Ok(png);
    }
    eprintln!(
        "[fastforge] msix: no logo_path configured and no PNG image found in {}; using a placeholder logo.",
        runner_icon.display()
    );
    Ok(solid_png(256, [0x02, 0x56, 0x9B, 0xFF]))
}

/// Windows-style quoted arguments, without invoking a shell or expanding
/// environment variables. Backslashes are literal unless before a quote.
fn split_signing_options(input: &str) -> Result<Vec<String>, PackageError> {
    let mut chars = input.chars().peekable();
    let mut args = Vec::new();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut arg = String::new();
        let mut quoted = false;
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() && !quoted {
                break;
            }
            if c == '\\' {
                let mut count = 0;
                while chars.peek() == Some(&'\\') {
                    count += 1;
                    chars.next();
                }
                if chars.peek() == Some(&'"') {
                    arg.extend(std::iter::repeat_n('\\', count / 2));
                    chars.next();
                    if count % 2 == 0 {
                        quoted = !quoted;
                    } else {
                        arg.push('"');
                    }
                } else {
                    arg.extend(std::iter::repeat_n('\\', count));
                }
            } else if c == '"' {
                chars.next();
                quoted = !quoted;
            } else {
                arg.push(c);
                chars.next();
            }
        }
        if quoted {
            return Err(PackageError::General(
                "Unclosed quote in signtool_options".into(),
            ));
        }
        args.push(arg);
    }
    Ok(args)
}

fn run(cmd: &mut Command) -> Result<(), PackageError> {
    let out = cmd.output().map_err(|e| {
        PackageError::MissingTool(format!("{}: {}", cmd.get_program().to_string_lossy(), e))
    })?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: cmd.get_program().to_string_lossy().into(),
            stderr: format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        });
    }
    Ok(())
}

/// Stage the bundle and resolve the native metadata before invoking SDK tools.
fn prepare_package(
    mc: &MsixMakeConfig,
    config: &PackageConfig,
    raw: &mut RawPackaging,
    root: &Path,
) -> Result<(), PackageError> {
    let mut mc = mc.clone();
    mc.display_name = mc
        .display_name
        .or_else(|| raw.settings.display_name.clone());
    mc.identity_name = mc.identity_name.or_else(|| raw.settings.app_id.clone());
    let publisher = raw
        .value("MSIX_PUBLISHER")
        .or_else(|| config.env_var("MSIX_PUBLISHER"))
        .or_else(|| mc.publisher.clone())
        .unwrap_or_else(|| "CN=Publisher".into());
    let manifest = MsixManifest::resolve(
        &mc,
        config,
        publisher,
        raw.settings.description.clone(),
        find_executable(&config.build_output_dir, &config.app_binary_name),
    );
    for (key, value) in [
        ("APP_ID", &manifest.identity_name),
        ("APP_DISPLAY_NAME", &manifest.display_name),
        ("PACKAGE_VERSION", &manifest.version),
        ("PACKAGE_ARCH", &manifest.architecture),
        ("EXECUTABLE_NAME", &manifest.executable),
        ("MSIX_PUBLISHER", &manifest.publisher),
    ] {
        raw.variables.insert(key.into(), value.clone());
    }
    raw.stage(config, root)?;
    let assets = root.join("Assets");
    if LOGO_ASSETS.iter().any(|name| !assets.join(name).is_file()) {
        let logo_path = mc.logo_path.as_deref().or(raw.settings.icon.as_deref());
        let logo = resolve_logo(
            logo_path,
            Path::new("windows/runner/resources/app_icon.ico"),
        )?;
        install_logo_assets(&logo, &assets)?;
    }
    let manifest_path = root.join("AppxManifest.xml");
    if let Some(template) = raw.template(&["AppxManifest.xml"])? {
        // Escape substitutions, preserving XML markup and namespace syntax.
        let variables = raw
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), xml_escape(v)))
            .collect();
        let text = std::fs::read_to_string(template)?;
        std::fs::write(
            manifest_path,
            fastforge_core::render_variables(&text, &variables),
        )?;
    } else if !manifest_path.is_file() {
        std::fs::write(manifest_path, manifest.render())?;
    }
    Ok(())
}

/// Generate only assets not supplied by the bundle or an overlay. Resize
/// without stretching and center the image on a transparent canvas.
fn install_logo_assets(logo: &[u8], assets: &Path) -> Result<(), PackageError> {
    let image = image::load_from_memory(logo)
        .map_err(|e| PackageError::General(format!("MSIX logo must be a valid PNG or ICO: {e}")))?;
    std::fs::create_dir_all(assets)?;
    for (name, width, height) in [
        (LOGO_ASSETS[0], 50, 50),
        (LOGO_ASSETS[1], 150, 150),
        (LOGO_ASSETS[2], 44, 44),
        (LOGO_ASSETS[3], 620, 300),
    ] {
        let path = assets.join(name);
        if path.is_file() {
            continue;
        }
        let resized = image.resize(width, height, image::imageops::FilterType::Lanczos3);
        let mut canvas = image::RgbaImage::new(width, height);
        image::imageops::overlay(
            &mut canvas,
            &resized.to_rgba8(),
            i64::from((width - resized.width()) / 2),
            i64::from((height - resized.height()) / 2),
        );
        canvas
            .save_with_format(&path, image::ImageFormat::Png)
            .map_err(|e| {
                PackageError::General(format!("Failed to write {}: {e}", path.display()))
            })?;
    }
    Ok(())
}

impl WindowsMsixPackager {
    fn create_natively(
        &self,
        mc: MsixMakeConfig,
        config: &PackageConfig,
        output_file: &Path,
    ) -> Result<(), PackageError> {
        let pkg_dir = config.packaging_dir();
        let mut raw = RawPackaging::load(config, &pkg_dir)?;
        if let Some(publisher) = &self.publisher {
            raw.variables
                .insert("MSIX_PUBLISHER".into(), publisher.clone());
        }
        let result = (|| {
            prepare_package(&mc, config, &mut raw, &pkg_dir)?;
            let mut command = Command::new("makeappx");
            command.args([
                "pack".as_ref(),
                "/d".as_ref(),
                pkg_dir.as_os_str(),
                "/p".as_ref(),
                output_file.as_os_str(),
                "/o".as_ref(),
            ]);
            raw.apply_env(&mut command, config);
            run(&mut command)?;
            self.sign_natively(&mc, config, &raw, output_file)
        })();
        if result.is_ok() {
            std::fs::remove_dir_all(&pkg_dir).ok();
        }
        result
    }

    fn sign_natively(
        &self,
        mc: &MsixMakeConfig,
        config: &PackageConfig,
        raw: &RawPackaging,
        output_file: &Path,
    ) -> Result<(), PackageError> {
        if is_true(&mc.store) {
            // Like the msix package: Store submissions are signed by the
            // Microsoft Store.
            return Ok(());
        }
        if is_false(&mc.sign_msix) {
            eprintln!(
                "[fastforge] warning: msix package left unsigned (sign_msix: false): {}",
                output_file.display()
            );
            return Ok(());
        }
        let certificate_path = self
            .certificate_path
            .clone()
            .or_else(|| raw.value("MSIX_CERTIFICATE_PATH"))
            .or_else(|| config.env_var("MSIX_CERTIFICATE_PATH"))
            .or_else(|| mc.certificate_path.clone());
        let certificate_password = self
            .certificate_password
            .clone()
            .or_else(|| raw.value("MSIX_CERTIFICATE_PASSWORD"))
            .or_else(|| config.env_var("MSIX_CERTIFICATE_PASSWORD"))
            .or_else(|| mc.certificate_password.clone());

        if let Some(signtool_options) = &mc.signtool_options {
            let mut args: Vec<String> = vec!["sign".to_string()];
            args.extend(split_signing_options(signtool_options)?);
            args.push(output_file.display().to_string());
            let mut command = Command::new("signtool");
            command.args(&args);
            raw.apply_env(&mut command, config);
            run(&mut command)
        } else if let Some(cert) = &certificate_path {
            if !Path::new(cert).is_file() {
                return Err(PackageError::NotFound(format!(
                    "The file certificate not found in: {}",
                    cert
                )));
            }
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
            let mut command = Command::new("signtool");
            command.args(&args);
            raw.apply_env(&mut command, config);
            run(&mut command)
        } else {
            eprintln!(
                "[fastforge] warning: msix package left UNSIGNED: {}\n  \
                 Windows will refuse to install it until it is signed. Configure \
                 `certificate_path`/`certificate_password` (or `signtool_options`) in \
                 windows/packaging/msix/make_config.yaml, or set MSIX_CERTIFICATE_PATH \
                 and MSIX_CERTIFICATE_PASSWORD in the environment.",
                output_file.display()
            );
            Ok(())
        }
    }
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
        let mut make_config = MsixMakeConfig::load()?;
        // Explicit packager settings override make_config.yaml.
        if self.certificate_path.is_some() {
            make_config.certificate_path = self.certificate_path.clone();
        }
        if self.certificate_password.is_some() {
            make_config.certificate_password = self.certificate_password.clone();
        }
        if self.publisher.is_some() {
            make_config.publisher = self.publisher.clone();
        }

        let pubspec = PubspecMsix::load();
        let make_config = make_config.or(pubspec.msix_config);
        if is_true(&make_config.install_certificate) || is_true(&make_config.debug) {
            eprintln!(
                "[fastforge] msix: debug/install_certificate are Dart-only options; install certificates separately for native packaging."
            );
        }
        let output_file = config.output_file();
        self.create_natively(make_config, config, &output_file)?;
        config.resolve_result(output_file)
    }
}

#[cfg(test)]
mod tests {
    use super::super::common::tests::{config, write};
    use super::*;
    use fastforge_core::ProjectSettings;
    use std::path::PathBuf;

    #[test]
    fn raw_manifest_escapes_values_and_preserves_supplied_assets() {
        let tmp = tempfile::tempdir().unwrap();
        let config = config(tmp.path(), "msix");
        write(&config.build_output_dir.join("runner.exe"), "runner");
        let dir = tmp.path().join("packaging/msix");
        write(
            &dir.join("AppxManifest.xml"),
            "<Package><Identity Name=\"${APP_ID}\" Version=\"${PACKAGE_VERSION}\" ProcessorArchitecture=\"${PACKAGE_ARCH}\" Publisher=\"${MSIX_PUBLISHER}\"/><Name>${APP_DISPLAY_NAME}</Name><Exe>${EXECUTABLE_NAME}</Exe></Package>",
        );
        write(
            &dir.parent()
                .unwrap()
                .join("shared/files/Assets/StoreLogo.png"),
            b"shared\0",
        );
        write(&dir.join("files/Assets/StoreLogo.png"), b"own\0");
        let root = tmp.path().join("stage");
        let mut raw = RawPackaging::new(
            dir,
            ProjectSettings {
                app_id: Some("dev.example.demo".into()),
                display_name: Some("Demo & Friends".into()),
                ..Default::default()
            },
            &config,
            &root,
        );
        let mc = MsixMakeConfig {
            publisher: Some("CN=Demo & Co".into()),
            ..Default::default()
        };
        // Keep this rendering test independent of the caller's signing env.
        raw.variables
            .insert("MSIX_PUBLISHER".into(), "CN=Demo & Co".into());
        prepare_package(&mc, &config, &mut raw, &root).unwrap();
        let xml = std::fs::read_to_string(root.join("AppxManifest.xml")).unwrap();
        assert!(xml.contains(
            "Name=\"dev.example.demo\" Version=\"1.2.3.0\" ProcessorArchitecture=\"arm64\""
        ));
        assert!(xml.contains("Publisher=\"CN=Demo &amp; Co\""));
        assert!(xml.contains("<Name>Demo &amp; Friends</Name><Exe>runner.exe</Exe>"));
        assert_eq!(
            std::fs::read(root.join("Assets/StoreLogo.png")).unwrap(),
            b"own\0"
        );
        for (name, width, height) in [
            (LOGO_ASSETS[1], 150, 150),
            (LOGO_ASSETS[2], 44, 44),
            (LOGO_ASSETS[3], 620, 300),
        ] {
            let image = image::open(root.join("Assets").join(name)).unwrap();
            assert_eq!((image.width(), image.height()), (width, height));
        }
    }

    #[test]
    fn generated_manifest_uses_project_metadata_and_legacy_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        let config = config(tmp.path(), "msix");
        write(&config.build_output_dir.join("runner.exe"), "runner");
        let root = tmp.path().join("stage");
        let mut raw = RawPackaging::new(
            tmp.path().join("msix"),
            ProjectSettings {
                app_id: Some("dev.example.demo".into()),
                display_name: Some("Project Name".into()),
                description: Some("Project description".into()),
                ..Default::default()
            },
            &config,
            &root,
        );
        prepare_package(&MsixMakeConfig::default(), &config, &mut raw, &root).unwrap();
        let xml = std::fs::read_to_string(root.join("AppxManifest.xml")).unwrap();
        assert!(xml.contains("<DisplayName>Project Name</DisplayName>"));
        assert!(xml.contains("<Description>Project description</Description>"));
        assert!(xml.contains("Name=\"dev.example.demo\""));
        std::fs::remove_dir_all(&root).unwrap();
        let mc = MsixMakeConfig {
            display_name: Some("Legacy Name".into()),
            identity_name: Some("dev.legacy.demo".into()),
            ..Default::default()
        };
        prepare_package(&mc, &config, &mut raw, &root).unwrap();
        let xml = std::fs::read_to_string(root.join("AppxManifest.xml")).unwrap();
        assert!(xml.contains("<DisplayName>Legacy Name</DisplayName>"));
        assert!(xml.contains("Name=\"dev.legacy.demo\""));
    }

    #[test]
    fn signing_options_preserve_quoted_paths_and_backslashes() {
        assert_eq!(
            split_signing_options(r#"/fd SHA256 /f "C:\My Certs\app.pfx" /p "pass word""#).unwrap(),
            [
                "/fd",
                "SHA256",
                "/f",
                r"C:\My Certs\app.pfx",
                "/p",
                "pass word"
            ]
        );
        assert!(split_signing_options("/f \"unclosed").is_err());
    }

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
            environment: Default::default(),
        }
    }

    fn full_make_config() -> MsixMakeConfig {
        serde_yaml::from_str(
            r#"
display_name: Hola Amigos
publisher_display_name: ACME Corp
identity_name: com.acme.hola
msix_version: 2.0.1.0
logo_path: assets/logo.png
trim_logo: false
languages: "en-us, zh-cn"
capabilities: "internetClient,microphone"
file_extension: ".txt,.md"
protocol_activation: "holaamigos://"
add_execution_alias: hola
enable_at_startup: true
store: false
debug: "true"
architecture: arm64
certificate_path: C:\certs\cert.pfx
certificate_password: 1234
publisher: CN=ACME
signtool_options: /v /fd SHA256
sign_msix: true
install_certificate: false
"#,
        )
        .unwrap()
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
        assert_eq!(
            detect_architecture(Path::new("build/windows/runner")),
            "x64"
        );
    }

    #[test]
    fn natural_yaml_scalars_are_accepted() {
        let mc = full_make_config();
        assert_eq!(mc.enable_at_startup.as_deref(), Some("true"));
        assert_eq!(mc.trim_logo.as_deref(), Some("false"));
        assert_eq!(mc.certificate_password.as_deref(), Some("1234"));
        assert_eq!(mc.msix_version.as_deref(), Some("2.0.1.0"));
    }

    #[test]
    fn execution_alias_resolution() {
        let alias = |yaml: &str| {
            serde_yaml::from_str::<MsixMakeConfig>(yaml)
                .unwrap()
                .resolved_execution_alias("hola_amigos")
        };
        assert_eq!(
            alias("add_execution_alias: true"),
            Some("holaamigos".into())
        );
        assert_eq!(alias("add_execution_alias: false"), None);
        assert_eq!(alias("add_execution_alias: hola"), Some("hola".into()));
        assert_eq!(
            alias("execution_alias: hi\nadd_execution_alias: true"),
            Some("hi".into())
        );
        assert_eq!(alias("{}"), None);
    }

    #[test]
    fn pubspec_detection_and_merge() {
        let pubspec: serde_yaml::Value = serde_yaml::from_str(
            r#"
name: hola_amigos
description: A friendly app
dev_dependencies:
  msix: ^3.16.6
msix_config:
  display_name: From Pubspec
  publisher: CN=Pubspec
  store: true
"#,
        )
        .unwrap();
        let info = PubspecMsix::from_value(&pubspec);
        let make_config: MsixMakeConfig =
            serde_yaml::from_str("display_name: From MakeConfig\n").unwrap();
        let merged = make_config.or(info.msix_config);
        assert_eq!(merged.display_name.as_deref(), Some("From MakeConfig"));
        assert_eq!(merged.publisher.as_deref(), Some("CN=Pubspec"));
        assert_eq!(merged.store.as_deref(), Some("true"));
    }

    #[test]
    fn manifest_with_full_config() {
        let manifest = MsixManifest::resolve(
            &full_make_config(),
            &test_config(),
            "CN=ACME".into(),
            Some("Desc".into()),
            "hola_amigos.exe".into(),
        )
        .render();
        assert!(manifest.contains(
            r#"<Identity Name="com.acme.hola" Publisher="CN=ACME" Version="2.0.1.0" ProcessorArchitecture="arm64"/>"#
        ));
        assert!(manifest.contains("<DisplayName>Hola Amigos</DisplayName>"));
        assert!(manifest.contains("<PublisherDisplayName>ACME Corp</PublisherDisplayName>"));
        assert!(manifest.contains("<Description>Desc</Description>"));
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
        assert!(manifest.contains(r#"Executable="hola_amigos.exe""#));
    }

    #[test]
    fn manifest_defaults_follow_msix_package() {
        let manifest = MsixManifest::resolve(
            &MsixMakeConfig::default(),
            &test_config(),
            "CN=Publisher".into(),
            None,
            "hola_amigos.exe".into(),
        );
        assert_eq!(manifest.display_name, "holaamigos");
        assert_eq!(manifest.identity_name, "com.flutter.holaamigos");
        assert_eq!(manifest.publisher_display_name, "com.flutter.holaamigos");
        assert_eq!(manifest.version, "1.2.3.0");
        assert_eq!(manifest.architecture, "x64");
        assert_eq!(manifest.description, "hola_amigos");
        let xml = manifest.render();
        assert!(xml.contains(r#"<Resource Language="en-us"/>"#));
        assert!(!xml.contains("<Extensions>"));
    }

    #[test]
    fn manifest_only_references_written_assets() {
        let xml = MsixManifest::resolve(
            &MsixMakeConfig::default(),
            &test_config(),
            "CN=Publisher".into(),
            None,
            "a.exe".into(),
        )
        .render();
        let mut rest = xml.as_str();
        let mut referenced = Vec::new();
        while let Some(pos) = rest.find("Assets\\") {
            let tail = &rest[pos + "Assets\\".len()..];
            let end = tail.find(['"', '<']).unwrap();
            referenced.push(tail[..end].to_string());
            rest = &tail[end..];
        }
        assert!(!referenced.is_empty());
        for asset in referenced {
            assert!(LOGO_ASSETS.contains(&asset.as_str()), "{asset}");
        }
    }

    #[test]
    fn placeholder_png_is_valid() {
        let png = solid_png(4, [1, 2, 3, 255]);
        assert!(png.starts_with(PNG_SIGNATURE));
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 4);
        // IHDR CRC
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&png[12..29]);
        assert_eq!(png[29..33], hasher.finalize().to_be_bytes());
        // IDAT decompresses to 4 rows of (1 + 4 * 4) bytes.
        let idat_len = u32::from_be_bytes(png[33..37].try_into().unwrap()) as usize;
        assert_eq!(&png[37..41], b"IDAT");
        let mut decoder = flate2::read::ZlibDecoder::new(&png[41..41 + idat_len]);
        let mut raw = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut raw).unwrap();
        assert_eq!(raw.len(), 4 * 17);
        assert!(png.ends_with(&[0xAE, 0x42, 0x60, 0x82])); // IEND CRC
    }

    #[test]
    fn extracts_largest_png_from_ico() {
        let small = solid_png(16, [0, 0, 0, 255]);
        let large = solid_png(32, [255, 0, 0, 255]);
        let bmp = vec![0u8; 40];
        let mut ico = vec![0, 0, 1, 0, 3, 0];
        let header_len = 6 + 3 * 16;
        let mut offset = header_len;
        let mut body = Vec::new();
        for (width, data) in [(16u8, &small), (0u8, &bmp), (32u8, &large)] {
            ico.extend_from_slice(&[width, width, 0, 0, 1, 0, 32, 0]);
            ico.extend_from_slice(&(data.len() as u32).to_le_bytes());
            ico.extend_from_slice(&(offset as u32).to_le_bytes());
            offset += data.len();
            body.extend_from_slice(data);
        }
        ico.extend_from_slice(&body);
        assert_eq!(png_from_ico(&ico), Some(large));
        assert_eq!(png_from_ico(b"not an icon"), None);
    }

    #[test]
    fn logo_resolution_fallbacks() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_logo(Some("/nonexistent/logo.png"), tmp.path()).unwrap_err();
        assert!(err.to_string().contains("Logo file not found"));

        let logo = tmp.path().join("logo.png");
        std::fs::write(&logo, b"png-bytes").unwrap();
        assert_eq!(
            resolve_logo(Some(&logo.display().to_string()), tmp.path()).unwrap(),
            b"png-bytes"
        );
        // No logo and no runner icon: placeholder PNG.
        let placeholder = resolve_logo(None, &tmp.path().join("missing.ico")).unwrap();
        assert!(placeholder.starts_with(PNG_SIGNATURE));
    }

    #[test]
    fn executable_detection() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(find_executable(tmp.path(), "hola"), "hola.exe");
        std::fs::write(tmp.path().join("runner.exe"), b"").unwrap();
        assert_eq!(find_executable(tmp.path(), "hola"), "runner.exe");
        std::fs::write(tmp.path().join("hola.exe"), b"").unwrap();
        assert_eq!(find_executable(tmp.path(), "hola"), "hola.exe");
    }
}
