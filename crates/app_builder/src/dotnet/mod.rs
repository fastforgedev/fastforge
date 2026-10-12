//! Native Windows (.NET / MSBuild) projects: WinUI 3, WPF, WinForms or
//! console apps built with `dotnet publish`, without a `pubspec.yaml`.

use fastforge_core::{AppBuilder, BuildConfig, BuildError, BuildResult, Platform};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::xcode::helpers::{extract_optional_str, extract_str, run};

/// Project file extensions `dotnet publish` accepts.
const PROJECT_EXTENSIONS: [&str; 4] = ["csproj", "fsproj", "vbproj", "sln"];

// ─── DotnetWindowsBuilder ─────────────────────────────────────────────────────

/// A builder for native Windows .NET projects.
///
/// Runs `dotnet publish` and reports the publish directory as the build
/// output, so the Windows packagers can stage it like a Flutter runner
/// directory.
///
/// # Minimum viable invocation
///
/// ```text
/// platform = "windows"
/// target   = "windows-dotnet"  (distinct from Flutter's "windows")
/// arguments = {
///   "project":            "apps/desktop/windows/MyApp/MyApp.csproj",
///   "configuration":      "Release",      // optional, default Release
///   "platform":           "x64",          // optional MSBuild Platform (x64 / ARM64)
///   "runtime-identifier": "win-x64",      // optional `-r`
///   "self-contained":     true,           // optional `--self-contained`
///   "properties":         {"Key": "Value"}, // optional `-p:Key=Value` pairs
///   "output":             "build/publish",  // optional `-o`
///   "extra-flags":        ["--nologo"],    // optional
/// }
/// ```
pub struct DotnetWindowsBuilder;

impl DotnetWindowsBuilder {
    /// `configuration`, defaulting to `Release`.
    pub fn configuration(config: &BuildConfig) -> &str {
        extract_optional_str(config, "configuration").unwrap_or("Release")
    }

    /// The package architecture (`x64` or `arm64`): `platform`, else the
    /// runtime identifier's suffix, else `x64`.
    pub fn architecture(config: &BuildConfig) -> &'static str {
        let from_platform = extract_optional_str(config, "platform")
            .map(str::to_ascii_lowercase)
            .and_then(|p| match p.as_str() {
                "arm64" => Some("arm64"),
                "x64" => Some("x64"),
                _ => None,
            });
        from_platform
            .or_else(|| {
                extract_optional_str(config, "runtime-identifier")
                    .map(str::to_ascii_lowercase)
                    .and_then(|rid| rid.rsplit_once('-').map(|(_, arch)| arch.to_string()))
                    .and_then(|arch| match arch.as_str() {
                        "arm64" => Some("arm64"),
                        "x64" => Some("x64"),
                        _ => None,
                    })
            })
            .unwrap_or("x64")
    }

    /// The publish directory: `output`, else
    /// `<project dir>/bin/fastforge/<configuration>/<arch>/publish`. The
    /// architecture segment lets the packagers detect `PACKAGE_ARCH` even
    /// when the executable cannot be inspected.
    pub fn publish_directory(config: &BuildConfig) -> Result<PathBuf, BuildError> {
        if let Some(output) = extract_optional_str(config, "output") {
            return Ok(PathBuf::from(output));
        }
        let project = extract_str(config, "project")?;
        let project_dir = Path::new(project).parent().unwrap_or(Path::new("."));
        let project_dir = if project_dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            project_dir
        };
        Ok(project_dir
            .join("bin")
            .join("fastforge")
            .join(Self::configuration(config))
            .join(Self::architecture(config))
            .join("publish"))
    }

    /// The `dotnet publish` arguments for `config`.
    pub fn publish_arguments(config: &BuildConfig) -> Result<Vec<String>, BuildError> {
        let project = extract_str(config, "project")?;
        let mut args = vec![
            "publish".to_string(),
            project.to_string(),
            "-c".to_string(),
            Self::configuration(config).to_string(),
        ];
        if let Some(rid) = extract_optional_str(config, "runtime-identifier") {
            args.extend(["-r".to_string(), rid.to_string()]);
        }
        match config.arguments.get("self-contained") {
            Some(Value::Bool(true)) => args.push("--self-contained".to_string()),
            Some(Value::Bool(false)) => args.push("--no-self-contained".to_string()),
            Some(Value::String(s)) if s.eq_ignore_ascii_case("true") => {
                args.push("--self-contained".to_string())
            }
            Some(Value::String(s)) if s.eq_ignore_ascii_case("false") => {
                args.push("--no-self-contained".to_string())
            }
            Some(Value::Null) => args.push("--self-contained".to_string()),
            Some(other) => {
                return Err(BuildError::InvalidArgument(format!(
                    "`self-contained` must be true or false, got {other}"
                )));
            }
            None => {}
        }
        if let Some(platform) = extract_optional_str(config, "platform") {
            args.push(format!("-p:Platform={platform}"));
        }
        if let Some(properties) = config.arguments.get("properties") {
            let Some(map) = properties.as_object() else {
                return Err(BuildError::InvalidArgument(
                    "`properties` must be an object of MSBuild property names to values".into(),
                ));
            };
            for (key, value) in map {
                let value = match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                args.push(format!("-p:{key}={value}"));
            }
        }
        args.extend([
            "-o".to_string(),
            Self::publish_directory(config)?.display().to_string(),
        ]);
        if let Some(flags) = config
            .arguments
            .get("extra-flags")
            .and_then(|v| v.as_array())
        {
            args.extend(flags.iter().filter_map(|f| f.as_str()).map(String::from));
        }
        Ok(args)
    }
}

impl AppBuilder for DotnetWindowsBuilder {
    fn name(&self) -> &str {
        "windows-dotnet"
    }

    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn matches(&self, platform: &Platform, target: Option<&str>) -> bool {
        platform == &Platform::Windows && target == Some("windows-dotnet")
    }

    fn is_supported_on_current_platform(&self) -> bool {
        Platform::current() == Some(Platform::Windows)
    }

    fn build_subcommand(&self) -> &str {
        "publish"
    }

    fn validate_arguments(&self, config: &BuildConfig) -> Result<(), BuildError> {
        let project = extract_str(config, "project")?;
        let extension = Path::new(project)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !extension.is_some_and(|e| PROJECT_EXTENSIONS.contains(&e.as_str())) {
            return Err(BuildError::InvalidArgument(format!(
                "`project` must be a .csproj, .fsproj, .vbproj or .sln file, got `{project}`"
            )));
        }
        Self::publish_arguments(config)?;
        Ok(())
    }

    fn outputs_directory(&self) -> bool {
        true
    }

    fn resolve_output_files(
        &self,
        config: &BuildConfig,
        _environment: Option<&HashMap<String, String>>,
    ) -> Result<(PathBuf, Vec<PathBuf>), BuildError> {
        Ok((Self::publish_directory(config)?, Vec::new()))
    }

    fn build_result(
        &self,
        config: BuildConfig,
        output_directory: PathBuf,
        output_files: Vec<PathBuf>,
        duration_ms: u128,
    ) -> BuildResult {
        BuildResult {
            config,
            output_directory,
            output_files,
            duration_ms,
            platform: Platform::Windows,
            target: Some("windows-dotnet".to_string()),
        }
    }
}

// ─── DotnetProject ────────────────────────────────────────────────────────────

/// Metadata of a .NET project file, read without MSBuild.
#[derive(Debug, Clone, PartialEq)]
pub struct DotnetProject {
    /// The project (or solution) file.
    pub path: PathBuf,
    /// The project file name without its extension.
    pub name: String,
    /// `<AssemblyName>` when set, else [`DotnetProject::name`]: the stem of
    /// the published executable.
    pub assembly_name: String,
    /// `<Version>` when set to a literal value.
    pub version: Option<String>,
    /// A `Package.appxmanifest` beside the project file, when present.
    pub manifest_path: Option<PathBuf>,
}

impl DotnetProject {
    /// Reads the project named by the `project` build argument.
    pub fn from_config(config: &BuildConfig) -> Result<Self, BuildError> {
        Self::load(Path::new(extract_str(config, "project")?))
    }

    pub fn load(path: &Path) -> Result<Self, BuildError> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                BuildError::InvalidArgument(format!(
                    "`project` has no file name: {}",
                    path.display()
                ))
            })?
            .to_string();
        if !path.is_file() {
            return Err(BuildError::ArtifactNotFound(format!(
                "Project file not found: {}",
                path.display()
            )));
        }
        let is_solution = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("sln"));
        let text = if is_solution {
            String::new()
        } else {
            std::fs::read_to_string(path)
                .map_err(|e| BuildError::Io(format!("{}: {}", path.display(), e)))?
        };
        let directory = path.parent().unwrap_or(Path::new("."));
        let manifest = directory.join("Package.appxmanifest");
        Ok(Self {
            path: path.to_path_buf(),
            assembly_name: property(&text, "AssemblyName").unwrap_or_else(|| name.clone()),
            version: property(&text, "Version"),
            name,
            manifest_path: manifest.is_file().then_some(manifest),
        })
    }
}

/// The first literal `<Name>value</Name>` of an MSBuild project file. Values
/// using MSBuild expressions (`$(...)`) are ignored.
fn property(text: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    let value = text[start..end].trim();
    (!value.is_empty() && !value.contains("$(")).then(|| value.to_string())
}

// ─── DotnetAppBuilder ─────────────────────────────────────────────────────────

/// High-level runner for native Windows .NET projects.
///
/// ```rust,ignore
/// let result = DotnetAppBuilder::default().build(
///     "windows",
///     Some("windows-dotnet"),
///     args_map,
///     None,
/// )?;
/// ```
pub struct DotnetAppBuilder {
    builder: DotnetWindowsBuilder,
}

impl Default for DotnetAppBuilder {
    fn default() -> Self {
        Self {
            builder: DotnetWindowsBuilder,
        }
    }
}

impl DotnetAppBuilder {
    pub fn build(
        &self,
        platform: &str,
        _target: Option<&str>,
        arguments: serde_json::Map<String, serde_json::Value>,
        environment: Option<HashMap<String, String>>,
    ) -> Result<BuildResult, BuildError> {
        if platform != "windows" {
            return Err(BuildError::UnsupportedBuilder(format!(
                "DotnetAppBuilder only handles platform=\"windows\", got \"{}\"",
                platform
            )));
        }
        if !self.builder.is_supported_on_current_platform() {
            return Err(BuildError::UnsupportedPlatform(
                "windows-dotnet is not supported on the current platform".into(),
            ));
        }

        let config = BuildConfig::new(arguments);
        self.builder.validate_arguments(&config)?;
        // Fail before invoking the SDK when the project file is missing.
        DotnetProject::from_config(&config)?;
        let args = DotnetWindowsBuilder::publish_arguments(&config)?;

        let mut cmd = Command::new("dotnet");
        cmd.args(&args);
        eprintln!("$ dotnet {}", args.join(" "));
        if let Some(env) = &environment {
            cmd.envs(env);
        }

        let start = std::time::Instant::now();
        run(&mut cmd)?;
        let duration = start.elapsed().as_millis();

        let (output_directory, output_files) = self
            .builder
            .resolve_output_files(&config, environment.as_ref())?;
        if !output_directory.is_dir() {
            return Err(BuildError::ArtifactNotFound(format!(
                "Publish directory not found: {}",
                output_directory.display()
            )));
        }
        Ok(self
            .builder
            .build_result(config, output_directory, output_files, duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};

    fn config(pairs: &[(&str, Value)]) -> BuildConfig {
        let mut args = Map::new();
        for (key, value) in pairs {
            args.insert(key.to_string(), value.clone());
        }
        BuildConfig::new(args)
    }

    #[test]
    fn matches_target() {
        assert!(DotnetWindowsBuilder.matches(&Platform::Windows, Some("windows-dotnet")));
        assert!(!DotnetWindowsBuilder.matches(&Platform::Windows, Some("windows")));
        assert!(!DotnetWindowsBuilder.matches(&Platform::Windows, None));
        assert!(!DotnetWindowsBuilder.matches(&Platform::Linux, Some("windows-dotnet")));
        assert!(DotnetWindowsBuilder.outputs_directory());
    }

    #[test]
    fn validate_requires_a_project_file() {
        assert!(
            DotnetWindowsBuilder
                .validate_arguments(&config(&[]))
                .is_err()
        );
        let err = DotnetWindowsBuilder
            .validate_arguments(&config(&[("project", json!("MyApp.xcodeproj"))]))
            .unwrap_err()
            .to_string();
        assert!(err.contains(".csproj"), "{err}");
        for project in [
            "MyApp.csproj",
            "MyApp.FSPROJ",
            "app/MyApp.vbproj",
            "All.sln",
        ] {
            DotnetWindowsBuilder
                .validate_arguments(&config(&[("project", json!(project))]))
                .unwrap_or_else(|e| panic!("{project}: {e}"));
        }
    }

    #[test]
    fn architecture_from_platform_then_runtime_identifier() {
        assert_eq!(DotnetWindowsBuilder::architecture(&config(&[])), "x64");
        assert_eq!(
            DotnetWindowsBuilder::architecture(&config(&[("platform", json!("ARM64"))])),
            "arm64"
        );
        assert_eq!(
            DotnetWindowsBuilder::architecture(&config(&[(
                "runtime-identifier",
                json!("win-arm64")
            )])),
            "arm64"
        );
        assert_eq!(
            DotnetWindowsBuilder::architecture(&config(&[
                ("platform", json!("x64")),
                ("runtime-identifier", json!("win-arm64")),
            ])),
            "x64"
        );
        assert_eq!(
            DotnetWindowsBuilder::architecture(&config(&[("platform", json!("AnyCPU"))])),
            "x64"
        );
    }

    #[test]
    fn publish_directory_defaults_under_the_project_bin() {
        let dir = DotnetWindowsBuilder::publish_directory(&config(&[
            ("project", json!("apps/win/MyApp/MyApp.csproj")),
            ("runtime-identifier", json!("win-arm64")),
        ]))
        .unwrap();
        assert_eq!(
            dir,
            Path::new("apps/win/MyApp/bin/fastforge/Release/arm64/publish")
        );
        let dir = DotnetWindowsBuilder::publish_directory(&config(&[
            ("project", json!("MyApp.csproj")),
            ("configuration", json!("Debug")),
        ]))
        .unwrap();
        assert_eq!(dir, Path::new("./bin/fastforge/Debug/x64/publish"));
        let dir = DotnetWindowsBuilder::publish_directory(&config(&[
            ("project", json!("MyApp.csproj")),
            ("output", json!("out/publish")),
        ]))
        .unwrap();
        assert_eq!(dir, Path::new("out/publish"));
    }

    #[test]
    fn publish_arguments_cover_every_option() {
        let args = DotnetWindowsBuilder::publish_arguments(&config(&[
            ("project", json!("MyApp/MyApp.csproj")),
            ("configuration", json!("Release")),
            ("platform", json!("x64")),
            ("runtime-identifier", json!("win-x64")),
            ("self-contained", json!(true)),
            (
                "properties",
                json!({"RustConfiguration": "release", "Number": 1}),
            ),
            ("extra-flags", json!(["--nologo", "-v:m"])),
        ]))
        .unwrap();
        // The output directory is joined with the host's separator.
        let output = ["bin", "fastforge", "Release", "x64", "publish"]
            .iter()
            .fold(Path::new("MyApp").to_path_buf(), |dir, part| dir.join(part))
            .display()
            .to_string();
        assert_eq!(
            args,
            [
                "publish",
                "MyApp/MyApp.csproj",
                "-c",
                "Release",
                "-r",
                "win-x64",
                "--self-contained",
                "-p:Platform=x64",
                "-p:RustConfiguration=release",
                "-p:Number=1",
                "-o",
                &output,
                "--nologo",
                "-v:m",
            ]
        );
        // `--flutter-build-args self-contained=false` arrives as a string.
        let args = DotnetWindowsBuilder::publish_arguments(&config(&[
            ("project", json!("MyApp.csproj")),
            ("self-contained", json!("false")),
        ]))
        .unwrap();
        assert!(args.contains(&"--no-self-contained".to_string()));
        assert!(!args.iter().any(|a| a == "-r"));
        let err = DotnetWindowsBuilder::publish_arguments(&config(&[
            ("project", json!("MyApp.csproj")),
            ("self-contained", json!("maybe")),
        ]))
        .unwrap_err()
        .to_string();
        assert!(err.contains("self-contained"), "{err}");
        let err = DotnetWindowsBuilder::publish_arguments(&config(&[
            ("project", json!("MyApp.csproj")),
            ("properties", json!("Key=Value")),
        ]))
        .unwrap_err()
        .to_string();
        assert!(err.contains("properties"), "{err}");
    }

    #[test]
    fn project_metadata_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("MyApp");
        std::fs::create_dir_all(&dir).unwrap();
        let csproj = dir.join("MyApp.csproj");
        std::fs::write(
            &csproj,
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>WinExe</OutputType>
    <AssemblyName>My.App</AssemblyName>
    <Version>2.1.0</Version>
  </PropertyGroup>
</Project>
"#,
        )
        .unwrap();
        let project = DotnetProject::load(&csproj).unwrap();
        assert_eq!(project.name, "MyApp");
        assert_eq!(project.assembly_name, "My.App");
        assert_eq!(project.version.as_deref(), Some("2.1.0"));
        assert_eq!(project.manifest_path, None);

        std::fs::write(dir.join("Package.appxmanifest"), "<Package/>").unwrap();
        std::fs::write(
            &csproj,
            "<Project><PropertyGroup><Version>$(AppVersion)</Version></PropertyGroup></Project>",
        )
        .unwrap();
        let project = DotnetProject::load(&csproj).unwrap();
        assert_eq!(project.assembly_name, "MyApp");
        assert_eq!(project.version, None);
        assert_eq!(
            project.manifest_path,
            Some(dir.join("Package.appxmanifest"))
        );

        let err = DotnetProject::load(&dir.join("Missing.csproj"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("Project file not found"), "{err}");
    }
}
