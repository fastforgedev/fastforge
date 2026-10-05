use crate::config::release::Release;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Parsed contents of `distribute_options.yaml`.
///
/// Deprecated: kept only for compatibility with the Dart CLI (`release`, and
/// the `output`/`variables` that `package`/`publish`/`build` read). Don't add
/// fields or features here; new configuration belongs elsewhere (for example
/// `.fastforge/` or `~/.fastforge/`), and new automation in workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributeOptions {
    /// Output directory for packaged artifacts (e.g. `dist/`).
    pub output: String,
    /// Global variable definitions.  These override environment variables
    /// and are themselves overridden by release- and job-level variables.
    ///
    /// The key `env` is accepted as a legacy alias for `variables`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
    /// Legacy alias kept for backward compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// Optional mustache template for artifact file names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_name: Option<String>,
    /// Named release definitions.
    #[serde(default)]
    pub releases: Vec<Release>,
}

impl DistributeOptions {
    /// Load and parse `distribute_options.yaml` from `path`.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let opts: DistributeOptions = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(opts)
    }

    /// Load `distribute_options.yaml` from the current working directory,
    /// returning a default (empty) config when the file does not exist.
    pub fn load() -> Result<Self> {
        let path = Path::new("distribute_options.yaml");
        if path.exists() {
            Self::from_file(path)
        } else {
            Ok(Self::default())
        }
    }

    /// Resolved variables: `variables`, or the legacy `env` key when
    /// `variables` is absent (Dart reads only one of them).
    pub fn resolved_variables(&self) -> HashMap<String, String> {
        self.variables
            .clone()
            .or_else(|| self.env.clone())
            .unwrap_or_default()
    }
}

impl Default for DistributeOptions {
    fn default() -> Self {
        Self {
            output: "dist/".to_string(),
            variables: None,
            env: None,
            artifact_name: None,
            releases: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const HELLO_WORLD_YAML: &str = r#"
output: dist/
variables:
  MY_KEY: my_value
releases:
  - name: dev-release
    jobs:
      - name: android-apk
        package:
          platform: android
          target: apk
          build_args:
            target-platform: android-arm,android-arm64
"#;

    const LEGACY_ENV_YAML: &str = r#"
output: dist/
env:
  LEGACY_KEY: legacy_value
releases: []
"#;

    const ARTIFACT_NAME_YAML: &str = r#"
output: dist/
artifact_name: "{{name}}-{{build_name}}.{{ext}}"
releases: []
"#;

    fn parse_yaml(content: &str) -> DistributeOptions {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        DistributeOptions::from_file(file.path()).unwrap()
    }

    #[test]
    fn test_parse_output_and_releases() {
        let opts = parse_yaml(HELLO_WORLD_YAML);
        assert_eq!(opts.output, "dist/");
        assert_eq!(opts.releases.len(), 1);
        assert_eq!(opts.releases[0].name, "dev-release");
        assert_eq!(opts.releases[0].jobs.len(), 1);
        let job = &opts.releases[0].jobs[0];
        assert_eq!(job.name, "android-apk");
        assert_eq!(job.package.platform, "android");
        assert_eq!(job.package.target, "apk");
    }

    #[test]
    fn test_variables_resolved() {
        let opts = parse_yaml(HELLO_WORLD_YAML);
        let vars = opts.resolved_variables();
        assert_eq!(vars.get("MY_KEY").map(String::as_str), Some("my_value"));
    }

    #[test]
    fn test_legacy_env_key() {
        let opts = parse_yaml(LEGACY_ENV_YAML);
        let vars = opts.resolved_variables();
        assert_eq!(
            vars.get("LEGACY_KEY").map(String::as_str),
            Some("legacy_value")
        );
    }

    #[test]
    fn test_artifact_name_parsed() {
        let opts = parse_yaml(ARTIFACT_NAME_YAML);
        assert_eq!(
            opts.artifact_name.as_deref(),
            Some("{{name}}-{{build_name}}.{{ext}}")
        );
    }

    #[test]
    fn test_default_when_no_file() {
        let opts = DistributeOptions::default();
        assert_eq!(opts.output, "dist/");
        assert!(opts.releases.is_empty());
    }

    #[test]
    fn test_build_args_optional() {
        let yaml = r#"
output: dist/
releases:
  - name: r
    jobs:
      - name: j
        package:
          platform: macos
          target: dmg
"#;
        let opts = parse_yaml(yaml);
        assert!(opts.releases[0].jobs[0].package.build_args.is_none());
    }
}
