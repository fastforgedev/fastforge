//! Variables fastforge provides while packaging.
//!
//! Packagers that accept a format's own raw files (for example a Debian
//! `control` file) render `${NAME}` references to these variables, and set
//! the same variables in the environment of the packaging tools, the
//! pre/post-package hooks and the `custom` packager's script.

use std::collections::BTreeMap;
use std::path::Path;

use serde_yaml::Value;

use crate::{PackageConfig, PackageError};

/// Variable name → value, ordered so rendered output and logs are stable.
pub type Variables = BTreeMap<String, String>;

/// Built-in variable names. `.fastforge/config.yaml` `env:` entries may not
/// reuse them; packagers add format-specific ones (`PACKAGE_ARCH`, ...).
pub const BUILTIN_VARIABLES: &[&str] = &[
    "APP_NAME",
    "APP_BINARY_NAME",
    "APP_VERSION",
    "APP_DISPLAY_NAME",
    "APP_DESCRIPTION",
    "APP_HOMEPAGE",
    "APP_ID",
    "BUILD_NAME",
    "BUILD_NUMBER",
    "BUILD_MODE",
    "FLAVOR",
    "CHANNEL",
    "PLATFORM",
    "PACKAGE_FORMAT",
    "PACKAGE_NAME",
    "PACKAGE_ARCH",
    "ARCH",
    "INSTALL_DIR",
    "RPM_RELEASE",
    "BUILD_OUTPUT_DIRECTORY",
    "OUTPUT_DIRECTORY",
    "OUTPUT_ARTIFACT_PATH",
    "PACKAGING_DIRECTORY",
];

/// Replaces `${NAME}` with the value of `NAME` when it is defined in
/// `variables`. Anything else is kept as written, so shell expansions such as
/// `$1`, `$(dirname "$0")` or `${HOME}`, RPM macros (`%{name}`) and desktop
/// field codes (`%U`) pass through untouched. `$${` renders a literal `${`.
pub fn render_variables(input: &str, variables: &Variables) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        if let Some(after) = tail.strip_prefix("$${") {
            out.push_str("${");
            rest = after;
            continue;
        }
        if let Some(body) = tail.strip_prefix("${")
            && let Some(end) = body.find('}')
            && let Some(value) = variables.get(&body[..end])
        {
            out.push_str(value);
            rest = &body[end + 1..];
            continue;
        }
        out.push('$');
        rest = &tail[1..];
    }
    out.push_str(rest);
    out
}

/// The variables to set in a process environment: those with a value. An
/// empty value (no flavor, no build number, ...) leaves the variable unset.
pub fn environment_variables(variables: &Variables) -> impl Iterator<Item = (&str, &str)> {
    variables
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| (key.as_str(), value.as_str()))
}

/// The parts of the project's `.fastforge/config.yaml` that packaging reads:
///
/// ```yaml
/// project:
///   display_name: Hello World
///   description: A short description   # falls back to pubspec.yaml
///   homepage: https://example.com       # falls back to pubspec.yaml
///   app_id: dev.example.hello
///   icon: assets/logo.png
/// env:
///   SIGNING_KEY_ID: ${SIGNING_KEY_ID}   # a whole-value reference reads the environment
///   SUPPORT_EMAIL: team@example.com
/// ```
#[derive(Debug, Clone, Default)]
pub struct ProjectSettings {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub app_id: Option<String>,
    pub icon: Option<String>,
    /// User-defined variables (`env:`), already resolved.
    pub env: Variables,
}

impl ProjectSettings {
    pub const PATH: &'static str = ".fastforge/config.yaml";

    /// Loads `.fastforge/config.yaml` and `pubspec.yaml` from the current
    /// directory. Missing files yield empty settings; a file that is not
    /// valid YAML, or an `env:` entry that redefines a built-in variable,
    /// is an error.
    pub fn load() -> Result<Self, PackageError> {
        Self::load_from(Path::new(Self::PATH), Path::new("pubspec.yaml"))
    }

    pub fn load_from(config: &Path, pubspec: &Path) -> Result<Self, PackageError> {
        let mut settings = match read_yaml(config)? {
            Some(value) => Self::from_value(&value, |name| std::env::var(name).ok())?,
            None => Self::default(),
        };
        if let Some(pubspec) = read_yaml(pubspec)? {
            if settings.description.is_none() {
                settings.description = string_at(&pubspec, "description");
            }
            if settings.homepage.is_none() {
                settings.homepage = string_at(&pubspec, "homepage");
            }
        }
        Ok(settings)
    }

    fn from_value(
        value: &Value,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, PackageError> {
        let project = value.get("project").cloned().unwrap_or(Value::Null);
        let mut env = Variables::new();
        if let Some(entries) = value.get("env").and_then(Value::as_mapping) {
            for (key, raw) in entries {
                let Some(key) = scalar(key) else { continue };
                if BUILTIN_VARIABLES.contains(&key.as_str()) {
                    return Err(PackageError::General(format!(
                        "`env.{key}` in {} redefines a built-in packaging variable",
                        Self::PATH
                    )));
                }
                let raw = scalar(raw).unwrap_or_default();
                let value = match env_reference(&raw) {
                    Some(name) => lookup(name).unwrap_or_default(),
                    None => raw,
                };
                env.insert(key, value);
            }
        }
        Ok(Self {
            display_name: string_at(&project, "display_name"),
            description: string_at(&project, "description"),
            homepage: string_at(&project, "homepage"),
            app_id: string_at(&project, "app_id"),
            icon: string_at(&project, "icon"),
            env,
        })
    }
}

impl PackageConfig {
    /// The variables shared by every packager: the app and build metadata,
    /// `project:` metadata and the user's `env:` entries.
    pub fn package_variables(&self, settings: &ProjectSettings) -> Variables {
        let mut version = self.app_version.splitn(2, '+');
        let build_name = version.next().unwrap_or_default().to_string();
        let build_number = version.next().unwrap_or_default().to_string();
        let absolute = |path: &Path| {
            std::path::absolute(path)
                .unwrap_or_else(|_| path.to_path_buf())
                .display()
                .to_string()
        };

        let mut variables = settings.env.clone();
        let mut set = |key: &str, value: String| {
            variables.insert(key.to_string(), value);
        };
        set("APP_NAME", self.app_name.clone());
        set("APP_BINARY_NAME", self.app_binary_name.clone());
        set("APP_VERSION", self.app_version.clone());
        set(
            "APP_DISPLAY_NAME",
            settings
                .display_name
                .clone()
                .unwrap_or_else(|| self.app_name.clone()),
        );
        set(
            "APP_DESCRIPTION",
            settings.description.clone().unwrap_or_default(),
        );
        set(
            "APP_HOMEPAGE",
            settings.homepage.clone().unwrap_or_default(),
        );
        set("APP_ID", settings.app_id.clone().unwrap_or_default());
        set("BUILD_NAME", build_name);
        set("BUILD_NUMBER", build_number);
        set("BUILD_MODE", self.build_mode.clone());
        set("FLAVOR", self.flavor.clone().unwrap_or_default());
        set("CHANNEL", self.channel.clone().unwrap_or_default());
        set("PLATFORM", self.platform.as_str().to_string());
        set("PACKAGE_FORMAT", self.package_format.clone());
        set("BUILD_OUTPUT_DIRECTORY", absolute(&self.build_output_dir));
        set("OUTPUT_DIRECTORY", absolute(&self.output_dir));
        variables
    }
}

fn read_yaml(path: &Path) -> Result<Option<Value>, PackageError> {
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackageError::General(format!("Failed to read {}: {}", path.display(), e)))?;
    serde_yaml::from_str(&content)
        .map(Some)
        .map_err(|e| PackageError::General(format!("Failed to parse {}: {}", path.display(), e)))
}

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(scalar)
        .filter(|s| !s.trim().is_empty())
}

/// `NAME` from a value that is exactly `${NAME}` (the convention
/// `.fastforge/config.yaml` already uses for store credentials).
fn env_reference(value: &str) -> Option<&str> {
    let name = value.trim().strip_prefix("${")?.strip_suffix('}')?;
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Platform;
    use std::path::PathBuf;

    fn vars(entries: &[(&str, &str)]) -> Variables {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn renders_defined_variables_only() {
        let v = vars(&[("APP_VERSION", "1.2.3"), ("APP_NAME", "demo")]);
        assert_eq!(
            render_variables("Version: ${APP_VERSION}\nPackage: ${APP_NAME}", &v),
            "Version: 1.2.3\nPackage: demo"
        );
        // Unknown names, shell expansions and other `$` uses pass through.
        assert_eq!(
            render_variables("${HOME}/x $1 $(dirname \"$0\") $ ${ ${APP_NAME", &v),
            "${HOME}/x $1 $(dirname \"$0\") $ ${ ${APP_NAME"
        );
        assert_eq!(
            render_variables("%{buildroot}%{_bindir} Exec=demo %U", &v),
            "%{buildroot}%{_bindir} Exec=demo %U"
        );
    }

    #[test]
    fn dollar_dollar_brace_is_a_literal() {
        let v = vars(&[("APP_NAME", "demo")]);
        assert_eq!(render_variables("$${APP_NAME}", &v), "${APP_NAME}");
        assert_eq!(render_variables("$$ $${", &v), "$$ ${");
        assert_eq!(render_variables("ünï ${APP_NAME}ç", &v), "ünï demoç");
    }

    #[test]
    fn settings_parse_project_and_env() {
        let value: Value = serde_yaml::from_str(
            r#"
project:
  name: ignored
  display_name: Hello World
  app_id: dev.example.hello
  icon: assets/logo.png
env:
  SUPPORT: team@example.com
  LEVEL: 3
  TOKEN: ${MY_TOKEN}
  MISSING: ${NOT_SET}
stores: {}
"#,
        )
        .unwrap();
        let settings = ProjectSettings::from_value(&value, |name| {
            (name == "MY_TOKEN").then(|| "secret".to_string())
        })
        .unwrap();
        assert_eq!(settings.display_name.as_deref(), Some("Hello World"));
        assert_eq!(settings.app_id.as_deref(), Some("dev.example.hello"));
        assert_eq!(settings.icon.as_deref(), Some("assets/logo.png"));
        assert_eq!(settings.description, None);
        assert_eq!(
            settings.env,
            vars(&[
                ("LEVEL", "3"),
                ("MISSING", ""),
                ("SUPPORT", "team@example.com"),
                ("TOKEN", "secret"),
            ])
        );
    }

    #[test]
    fn env_may_not_redefine_builtins() {
        let value: Value = serde_yaml::from_str("env:\n  APP_VERSION: 9.9.9\n").unwrap();
        let err = ProjectSettings::from_value(&value, |_| None).unwrap_err();
        assert!(err.to_string().contains("env.APP_VERSION"));
    }

    #[test]
    fn load_falls_back_to_pubspec_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.yaml");
        let pubspec = dir.path().join("pubspec.yaml");
        std::fs::write(&config, "project:\n  homepage: https://own.example\n").unwrap();
        std::fs::write(
            &pubspec,
            "name: demo\ndescription: From pubspec\nhomepage: https://pub.example\n",
        )
        .unwrap();
        let settings = ProjectSettings::load_from(&config, &pubspec).unwrap();
        assert_eq!(settings.description.as_deref(), Some("From pubspec"));
        assert_eq!(settings.homepage.as_deref(), Some("https://own.example"));

        let missing =
            ProjectSettings::load_from(&dir.path().join("none.yaml"), &dir.path().join("none"))
                .unwrap();
        assert!(missing.env.is_empty());
        assert_eq!(missing.description, None);
    }

    #[test]
    fn package_variables_cover_app_and_build() {
        let config = PackageConfig {
            app_name: "hello_world".into(),
            app_binary_name: "hello-world".into(),
            app_version: "1.2.3+4".into(),
            build_mode: "release".into(),
            platform: Platform::Linux,
            flavor: None,
            channel: Some("beta".into()),
            artifact_name: None,
            package_format: "deb".into(),
            is_installer: false,
            build_output_dir: PathBuf::from("/tmp/bundle"),
            build_output_files: vec![],
            output_dir: PathBuf::from("/tmp/dist"),
            environment: Default::default(),
        };
        let settings = ProjectSettings {
            env: vars(&[("SUPPORT", "team@example.com")]),
            ..Default::default()
        };
        let v = config.package_variables(&settings);
        assert_eq!(v["APP_NAME"], "hello_world");
        assert_eq!(v["APP_BINARY_NAME"], "hello-world");
        assert_eq!(v["APP_DISPLAY_NAME"], "hello_world");
        assert_eq!(v["BUILD_NAME"], "1.2.3");
        assert_eq!(v["BUILD_NUMBER"], "4");
        assert_eq!(v["FLAVOR"], "");
        assert_eq!(v["CHANNEL"], "beta");
        assert_eq!(v["PLATFORM"], "linux");
        assert_eq!(v["PACKAGE_FORMAT"], "deb");
        assert_eq!(v["BUILD_OUTPUT_DIRECTORY"], "/tmp/bundle");
        assert_eq!(v["SUPPORT"], "team@example.com");

        let unnumbered = PackageConfig {
            app_version: "1.2.3".into(),
            ..config
        };
        assert_eq!(unnumbered.package_variables(&settings)["BUILD_NUMBER"], "");
    }
}
