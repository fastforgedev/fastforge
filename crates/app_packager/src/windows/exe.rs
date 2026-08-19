use std::path::Path;
use std::process::Command;

use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};
use serde::Deserialize;

/// Builds a Windows `.exe` installer using Inno Setup (`iscc`), mirroring
/// Dart's `AppPackageMakerExe`.
///
/// Reads `windows/packaging/exe/make_config.yaml` when present (same schema
/// as Dart's `MakeExeConfig`); falls back to sensible defaults otherwise.
///
/// Requires Inno Setup (`iscc`) to be on `%PATH%`, in the default install
/// location, or pointed to by the `INNO_SETUP_PATH` environment variable
/// (Windows only).
pub struct WindowsExePackager;

/// Maps locale codes to Inno Setup language metadata:
/// `(locale, language name, .isl file)`. Mirrors Dart's
/// `_localeToLanguageFile` + `[Languages]` template block.
const LOCALE_LANGUAGES: &[(&str, &str, &str)] = &[
    ("en", "english", "Default.isl"),
    ("hy", "armenian", "Armenian.isl"),
    ("bg", "bulgarian", "Bulgarian.isl"),
    ("ca", "catalan", "Catalan.isl"),
    ("zh", "chinesesimplified", "ChineseSimplified.isl"),
    ("co", "corsican", "Corsican.isl"),
    ("cs", "czech", "Czech.isl"),
    ("da", "danish", "Danish.isl"),
    ("nl", "dutch", "Dutch.isl"),
    ("fi", "finnish", "Finnish.isl"),
    ("fr", "french", "French.isl"),
    ("de", "german", "German.isl"),
    ("he", "hebrew", "Hebrew.isl"),
    ("is", "icelandic", "Icelandic.isl"),
    ("it", "italian", "Italian.isl"),
    ("ja", "japanese", "Japanese.isl"),
    ("no", "norwegian", "Norwegian.isl"),
    ("pl", "polish", "Polish.isl"),
    ("pt", "portuguese", "Portuguese.isl"),
    ("ru", "russian", "Russian.isl"),
    ("sk", "slovak", "Slovak.isl"),
    ("sl", "slovenian", "Slovenian.isl"),
    ("es", "spanish", "Spanish.isl"),
    ("tr", "turkish", "Turkish.isl"),
    ("uk", "ukrainian", "Ukrainian.isl"),
];

/// Schema of `windows/packaging/exe/make_config.yaml`, mirroring Dart's
/// `MakeExeConfig.fromJson`.
#[derive(Debug, Default, Deserialize)]
pub struct ExeMakeConfig {
    pub script_template: Option<String>,
    #[serde(alias = "appId")]
    pub app_id: Option<String>,
    pub executable_name: Option<String>,
    pub display_name: Option<String>,
    #[serde(alias = "appPublisher")]
    pub publisher_name: Option<String>,
    #[serde(alias = "appPublisherUrl")]
    pub publisher_url: Option<String>,
    pub create_desktop_icon: Option<bool>,
    pub launch_at_startup: Option<bool>,
    pub install_dir_name: Option<String>,
    pub setup_icon_file: Option<String>,
    pub privileges_required: Option<String>,
    pub locales: Option<Vec<String>>,
    pub architectures_allowed: Option<String>,
    pub architectures_install_in_64bit_mode: Option<String>,
}

impl ExeMakeConfig {
    fn load() -> Result<Self, PackageError> {
        let path = Path::new("windows/packaging/exe/make_config.yaml");
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

/// Resolves the path to `ISCC.exe` using the same order as Dart's
/// `InnoSetupCompiler`:
/// 1. `INNO_SETUP_PATH` environment variable (a directory containing ISCC.exe)
/// 2. The default install path (`C:\Program Files (x86)\Inno Setup 6`)
/// 3. `iscc` found in `PATH`
fn resolve_iscc_path() -> String {
    if let Ok(env_path) = std::env::var("INNO_SETUP_PATH")
        && !env_path.is_empty()
    {
        let iscc = std::path::Path::new(&env_path).join("ISCC.exe");
        if iscc.exists() {
            return iscc.display().to_string();
        }
    }
    let default_iscc =
        std::path::Path::new(r"C:\Program Files (x86)\Inno Setup 6").join("ISCC.exe");
    if default_iscc.exists() {
        return default_iscc.display().to_string();
    }
    "iscc".to_string()
}

/// Filters locales to those whose `.isl` files exist at the resolved Inno
/// Setup location, mirroring Dart's `_getAvailableLocales`. When iscc is only
/// available via `PATH` the directory is unknown, so all locales are kept.
fn available_locales(locales: &[String], iscc_path: &str) -> Vec<String> {
    if locales.is_empty() {
        return vec!["en".to_string()];
    }
    if iscc_path == "iscc" {
        return locales.to_vec();
    }
    let inno_dir = Path::new(iscc_path).parent().unwrap_or(Path::new("."));
    let mut available = Vec::new();
    for locale in locales {
        match LOCALE_LANGUAGES.iter().find(|(code, _, _)| code == locale) {
            None => available.push(locale.clone()),
            Some((code, _, isl)) => {
                let isl_path = if *code == "en" {
                    inno_dir.join(isl)
                } else {
                    inno_dir.join("Languages").join(isl)
                };
                if isl_path.exists() {
                    available.push(locale.clone());
                }
            }
        }
    }
    if available.is_empty() {
        vec!["en".to_string()]
    } else {
        available
    }
}

/// Renders the `[Languages]` section entries for the given locales.
fn render_languages(locales: &[String]) -> String {
    locales
        .iter()
        .filter_map(|locale| {
            LOCALE_LANGUAGES
                .iter()
                .find(|(code, _, _)| code == locale)
                .map(|(code, name, isl)| {
                    if *code == "en" {
                        format!("Name: \"{}\"; MessagesFile: \"compiler:{}\"", name, isl)
                    } else {
                        format!(
                            "Name: \"{}\"; MessagesFile: \"compiler:Languages\\{}\"",
                            name, isl
                        )
                    }
                })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct IssVariables {
    app_id: String,
    app_version: String,
    display_name: String,
    publisher_name: String,
    publisher_url: String,
    install_dir_name: String,
    output_base_filename: String,
    setup_icon_file: String,
    privileges_required: String,
    architectures_allowed: String,
    architectures_install_in_64bit_mode: String,
    source_dir: String,
    executable_name: String,
    create_desktop_icon: bool,
    launch_at_startup: bool,
    locales: Vec<String>,
}

impl IssVariables {
    /// Renders the default `.iss` script, producing the same output as Dart's
    /// Liquid `_template`.
    fn render_default(&self) -> String {
        let run_flags = if self.privileges_required == "admin" {
            "runascurrentuser nowait postinstall skipifsilent"
        } else {
            " nowait postinstall skipifsilent"
        };
        format!(
            "[Setup]\n\
             AppId={app_id}\n\
             AppVersion={app_version}\n\
             AppName={display_name}\n\
             AppPublisher={publisher_name}\n\
             AppPublisherURL={publisher_url}\n\
             AppSupportURL={publisher_url}\n\
             AppUpdatesURL={publisher_url}\n\
             DefaultDirName={install_dir_name}\n\
             DisableProgramGroupPage=yes\n\
             OutputDir=.\n\
             OutputBaseFilename={output_base_filename}\n\
             Compression=lzma\n\
             SolidCompression=yes\n\
             SetupIconFile={setup_icon_file}\n\
             WizardStyle=modern\n\
             PrivilegesRequired={privileges_required}\n\
             ArchitecturesAllowed={architectures_allowed}\n\
             ArchitecturesInstallIn64BitMode={architectures_install_in_64bit_mode}\n\
             \n\
             [Languages]\n\
             {languages}\n\
             \n\
             [Tasks]\n\
             Name: \"desktopicon\"; Description: \"{{cm:CreateDesktopIcon}}\"; GroupDescription: \"{{cm:AdditionalIcons}}\"; Flags: {desktop_icon_flags}\n\
             Name: \"launchAtStartup\"; Description: \"{{cm:AutoStartProgram,{display_name}}}\"; GroupDescription: \"{{cm:AdditionalIcons}}\"; Flags: {startup_flags}\n\
             [Files]\n\
             Source: \"{source_dir}\\*\"; DestDir: \"{{app}}\"; Flags: ignoreversion recursesubdirs createallsubdirs\n\
             ; NOTE: Don't use \"Flags: ignoreversion\" on any shared system files\n\
             \n\
             [Icons]\n\
             Name: \"{{autoprograms}}\\{display_name}\"; Filename: \"{{app}}\\{executable_name}\"\n\
             Name: \"{{autodesktop}}\\{display_name}\"; Filename: \"{{app}}\\{executable_name}\"; Tasks: desktopicon\n\
             Name: \"{{userstartup}}\\{display_name}\"; Filename: \"{{app}}\\{executable_name}\"; WorkingDir: \"{{app}}\"; Tasks: launchAtStartup\n\
             [Run]\n\
             Filename: \"{{app}}\\{executable_name}\"; Description: \"{{cm:LaunchProgram,{display_name}}}\"; Flags: {run_flags}\n",
            app_id = self.app_id,
            app_version = self.app_version,
            display_name = self.display_name,
            publisher_name = self.publisher_name,
            publisher_url = self.publisher_url,
            install_dir_name = self.install_dir_name,
            output_base_filename = self.output_base_filename,
            setup_icon_file = self.setup_icon_file,
            privileges_required = self.privileges_required,
            architectures_allowed = self.architectures_allowed,
            architectures_install_in_64bit_mode = self.architectures_install_in_64bit_mode,
            languages = render_languages(&self.locales),
            desktop_icon_flags = if self.create_desktop_icon {
                "checkedonce"
            } else {
                "unchecked"
            },
            startup_flags = if self.launch_at_startup {
                "checkedonce"
            } else {
                "unchecked"
            },
            source_dir = self.source_dir,
            executable_name = self.executable_name,
            run_flags = run_flags,
        )
    }

    /// Substitutes `{{VAR}}` placeholders in a user-provided template.
    /// (Liquid `{% %}` logic blocks are not supported; simple variable
    /// substitution covers the common custom-template case.)
    fn render_template(&self, template: &str) -> String {
        let pairs: Vec<(&str, String)> = vec![
            ("APP_ID", self.app_id.clone()),
            ("APP_VERSION", self.app_version.clone()),
            ("DISPLAY_NAME", self.display_name.clone()),
            ("PUBLISHER_NAME", self.publisher_name.clone()),
            ("PUBLISHER_URL", self.publisher_url.clone()),
            ("INSTALL_DIR_NAME", self.install_dir_name.clone()),
            ("OUTPUT_BASE_FILENAME", self.output_base_filename.clone()),
            ("SETUP_ICON_FILE", self.setup_icon_file.clone()),
            ("PRIVILEGES_REQUIRED", self.privileges_required.clone()),
            (
                "ARCHITECTURES_ALLOWED",
                self.architectures_allowed.clone(),
            ),
            (
                "ARCHITECTURES_INSTALL_IN_64BIT_MODE",
                self.architectures_install_in_64bit_mode.clone(),
            ),
            ("SOURCE_DIR", self.source_dir.clone()),
            ("EXECUTABLE_NAME", self.executable_name.clone()),
        ];
        let mut output = template.to_string();
        for (key, value) in pairs {
            output = output.replace(&format!("{{{{{}}}}}", key), &value);
        }
        output
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

impl AppPackager for WindowsExePackager {
    fn name(&self) -> &str {
        "exe"
    }

    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn package_format(&self) -> &str {
        "exe"
    }

    #[cfg(not(target_os = "windows"))]
    fn is_supported_on_current_platform(&self) -> bool {
        false
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let make_config = ExeMakeConfig::load()?;
        let pkg_dir = config.packaging_dir();

        // Copy the flutter build output into the packaging directory
        run(Command::new("xcopy").args([
            "/E",
            "/I",
            "/Q",
            &config.build_output_dir.display().to_string(),
            &pkg_dir.display().to_string(),
        ]))?;

        let output_file = config.output_file();
        let output_base_filename = output_file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        // Default executable: the first .exe in the packaging directory
        // (mirrors Dart's `defaultExecutableName`).
        let executable_name = match &make_config.executable_name {
            Some(name) => name.clone(),
            None => std::fs::read_dir(&pkg_dir)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|x| x == "exe"))
                .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
                .unwrap_or_else(|| format!("{}.exe", config.app_binary_name)),
        };

        // Absolute setup icon path (mirrors Dart, which joins with cwd)
        let setup_icon_file = make_config
            .setup_icon_file
            .as_ref()
            .map(|icon| {
                std::env::current_dir()
                    .map(|cwd| cwd.join(icon).display().to_string())
                    .unwrap_or_else(|_| icon.clone())
            })
            .unwrap_or_default();

        let iscc_path = resolve_iscc_path();
        let locales = available_locales(
            &make_config
                .locales
                .clone()
                .unwrap_or_else(|| vec!["en".to_string()]),
            &iscc_path,
        );

        let variables = IssVariables {
            app_id: make_config
                .app_id
                .clone()
                .unwrap_or_else(|| config.app_name.clone()),
            app_version: config.app_version.clone(),
            display_name: make_config
                .display_name
                .clone()
                .unwrap_or_else(|| config.app_name.clone()),
            publisher_name: make_config.publisher_name.clone().unwrap_or_default(),
            publisher_url: make_config.publisher_url.clone().unwrap_or_default(),
            install_dir_name: make_config
                .install_dir_name
                .clone()
                .unwrap_or_else(|| format!("{{autopf64}}\\{}", config.app_name)),
            output_base_filename,
            setup_icon_file,
            privileges_required: make_config
                .privileges_required
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            architectures_allowed: make_config
                .architectures_allowed
                .clone()
                .unwrap_or_else(|| "x64compatible".to_string()),
            architectures_install_in_64bit_mode: make_config
                .architectures_install_in_64bit_mode
                .clone()
                .unwrap_or_else(|| "x64compatible".to_string()),
            source_dir: pkg_dir
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default(),
            executable_name,
            create_desktop_icon: make_config.create_desktop_icon.unwrap_or(false),
            launch_at_startup: make_config.launch_at_startup.unwrap_or(false),
            locales,
        };

        // Render the script (custom template when configured, mirroring
        // Dart's `script_template` support)
        let content = match &make_config.script_template {
            Some(template_name) => {
                let template_path = Path::new("windows/packaging/exe").join(template_name);
                let template = std::fs::read_to_string(&template_path).map_err(|e| {
                    PackageError::General(format!(
                        "Failed to read script template {}: {}",
                        template_path.display(),
                        e
                    ))
                })?;
                variables.render_template(&template)
            }
            None => variables.render_default(),
        };

        // The .iss file sits next to the packaging directory (in the version
        // output dir), so `OutputDir=.` and `SOURCE_DIR\*` resolve correctly.
        // A UTF-8 BOM is prepended, mirroring Dart.
        let iss_path = pkg_dir.with_extension("iss");
        std::fs::write(&iss_path, format!("\u{FEFF}{}", content))?;

        run(Command::new(&iscc_path).arg(&iss_path))?;

        std::fs::remove_file(&iss_path).ok();
        std::fs::remove_dir_all(&pkg_dir).ok();
        Ok(PackageResult {
            artifacts: vec![output_file],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_variables() -> IssVariables {
        IssVariables {
            app_id: "6BCF1E88-6912-4D77-8FE8-B10A2A1B5A0E".into(),
            app_version: "1.2.3+4".into(),
            display_name: "Hola Amigos".into(),
            publisher_name: "ACME".into(),
            publisher_url: "https://example.com".into(),
            install_dir_name: "{autopf64}\\hola_amigos".into(),
            output_base_filename: "hola_amigos-1.2.3+4-windows-setup".into(),
            setup_icon_file: "C:\\proj\\icon.ico".into(),
            privileges_required: "admin".into(),
            architectures_allowed: "x64compatible".into(),
            architectures_install_in_64bit_mode: "x64compatible".into(),
            source_dir: "hola_amigos-1.2.3+4-windows-setup_exe".into(),
            executable_name: "hola_amigos.exe".into(),
            create_desktop_icon: true,
            launch_at_startup: false,
            locales: vec!["en".into(), "zh".into()],
        }
    }

    #[test]
    fn default_script_contains_all_sections() {
        let iss = test_variables().render_default();
        assert!(iss.contains("AppId=6BCF1E88-6912-4D77-8FE8-B10A2A1B5A0E"));
        assert!(iss.contains("AppVersion=1.2.3+4"));
        assert!(iss.contains("AppName=Hola Amigos"));
        assert!(iss.contains("AppPublisher=ACME"));
        assert!(iss.contains("AppPublisherURL=https://example.com"));
        assert!(iss.contains("DefaultDirName={autopf64}\\hola_amigos"));
        assert!(iss.contains("SetupIconFile=C:\\proj\\icon.ico"));
        assert!(iss.contains("PrivilegesRequired=admin"));
        assert!(iss.contains("ArchitecturesAllowed=x64compatible"));
        assert!(iss.contains("Name: \"english\"; MessagesFile: \"compiler:Default.isl\""));
        assert!(iss.contains(
            "Name: \"chinesesimplified\"; MessagesFile: \"compiler:Languages\\ChineseSimplified.isl\""
        ));
        assert!(iss.contains("Name: \"desktopicon\""));
        assert!(iss.contains("Flags: checkedonce\n"));
        assert!(iss.contains("Name: \"launchAtStartup\""));
        assert!(iss.contains("Source: \"hola_amigos-1.2.3+4-windows-setup_exe\\*\""));
        assert!(iss.contains("Filename: \"{app}\\hola_amigos.exe\""));
        assert!(iss.contains("Flags: runascurrentuser nowait postinstall skipifsilent"));
    }

    #[test]
    fn non_admin_run_flags() {
        let mut vars = test_variables();
        vars.privileges_required = "none".into();
        let iss = vars.render_default();
        assert!(iss.contains("Flags:  nowait postinstall skipifsilent"));
        assert!(!iss.contains("runascurrentuser"));
    }

    #[test]
    fn custom_template_substitution() {
        let iss = test_variables().render_template(
            "[Setup]\nAppId={{APP_ID}}\nAppName={{DISPLAY_NAME}}\nSource: \"{{SOURCE_DIR}}\\*\"",
        );
        assert_eq!(
            iss,
            "[Setup]\nAppId=6BCF1E88-6912-4D77-8FE8-B10A2A1B5A0E\nAppName=Hola Amigos\nSource: \"hola_amigos-1.2.3+4-windows-setup_exe\\*\""
        );
    }

    #[test]
    fn locales_fall_back_to_english() {
        assert_eq!(available_locales(&[], "iscc"), vec!["en".to_string()]);
        // With PATH-resolved iscc all locales are kept
        assert_eq!(
            available_locales(&["en".to_string(), "zh".to_string()], "iscc"),
            vec!["en".to_string(), "zh".to_string()]
        );
    }
}
