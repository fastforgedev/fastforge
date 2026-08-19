use crate::model::Platform;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ── Types ─────────────────────────────────────────────────────────────────────

fn render_artifact_name(config: &PackageConfig) -> String {
    let build_name = config
        .app_version
        .split('+')
        .next()
        .unwrap_or(&config.app_version)
        .to_string();
    let build_number = config.app_version.split('+').nth(1).map(|s| s.to_string());

    let mut name = config.app_name.clone();
    if let Some(channel) = &config.channel {
        // Channel variant mirrors Dart's `_kArtifactNameWithChannel`:
        // the channel replaces the flavor segment.
        name.push('-');
        name.push_str(channel);
    } else if let Some(flavor) = &config.flavor {
        name.push('-');
        name.push_str(flavor);
    }
    name.push('-');
    name.push_str(&build_name);
    if let Some(number) = &build_number {
        name.push('+');
        name.push_str(number);
    }
    if config.build_mode == "profile" {
        name.push('-');
        name.push_str(&config.build_mode);
    }
    name.push('-');
    name.push_str(config.platform.as_str());
    if config.is_installer {
        name.push_str("-setup");
    }
    if !config.package_format.is_empty() {
        name.push('.');
        name.push_str(&config.package_format);
    }
    name
}

/// Renders a mustache-style artifact-name template, mirroring the template
/// variables supported by Dart's `MakeConfig.outputArtifactPath`:
/// `{{name}}`, `{{version}}`, `{{build_name}}`, `{{build_number}}`,
/// `{{build_mode}}`, `{{platform}}`, `{{flavor}}`, `{{channel}}`, `{{ext}}`,
/// plus boolean sections `{{#is_installer}}`, `{{#is_profile}}`,
/// `{{#has_build_number}}` and value sections like `{{#flavor}}`/`{{#ext}}`.
fn render_artifact_template(template: &str, config: &PackageConfig) -> String {
    let build_name = config
        .app_version
        .split('+')
        .next()
        .unwrap_or(&config.app_version)
        .to_string();
    let build_number = config.app_version.split('+').nth(1).map(|s| s.to_string());
    let ext = if config.package_format.is_empty() {
        None
    } else {
        Some(config.package_format.clone())
    };

    let lookup = |key: &str| -> Option<String> {
        match key {
            "name" => Some(config.app_name.clone()),
            "version" => Some(config.app_version.clone()),
            "build_name" => Some(build_name.clone()),
            "build_number" => build_number.clone(),
            "build_mode" => Some(config.build_mode.clone()),
            "platform" => Some(config.platform.as_str().to_string()),
            "flavor" => config.flavor.clone(),
            "channel" => config.channel.clone(),
            "ext" => ext.clone(),
            _ => None,
        }
    };
    let truthy = |key: &str| -> bool {
        match key {
            "is_installer" => config.is_installer,
            "is_profile" => config.build_mode == "profile",
            "has_build_number" => build_number.is_some(),
            other => lookup(other).is_some_and(|v| !v.is_empty()),
        }
    };

    render_mustache(template, &lookup, &truthy)
}

/// Minimal mustache renderer supporting `{{var}}` interpolation and
/// `{{#section}}...{{/section}}` conditional sections (nested allowed).
fn render_mustache(
    template: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    truthy: &dyn Fn(&str) -> bool,
) -> String {
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            output.push_str(&rest[start..]);
            return output;
        };
        let tag = after[..end].trim();
        rest = &after[end + 2..];
        if let Some(section) = tag.strip_prefix('#') {
            let close = format!("{{{{/{}}}}}", section);
            if let Some(close_pos) = rest.find(&close) {
                let inner = &rest[..close_pos];
                rest = &rest[close_pos + close.len()..];
                if truthy(section) {
                    output.push_str(&render_mustache(inner, lookup, truthy));
                }
            }
        } else if !tag.starts_with('/')
            && !tag.starts_with('^')
            && let Some(value) = lookup(tag)
        {
            output.push_str(&value);
        }
    }
    output.push_str(rest);
    output
}

#[derive(Debug, Clone)]
pub struct PackageConfig {
    pub app_name: String,
    pub app_binary_name: String,
    pub app_version: String,
    pub build_mode: String,
    pub platform: Platform,
    pub flavor: Option<String>,
    pub channel: Option<String>,
    pub artifact_name: Option<String>,
    pub package_format: String,
    pub is_installer: bool,
    pub build_output_dir: PathBuf,
    pub build_output_files: Vec<PathBuf>,
    pub output_dir: PathBuf,
}

impl PackageConfig {
    pub fn output_file_name(&self) -> String {
        if let Some(template) = &self.artifact_name {
            return render_artifact_template(template, self);
        }
        render_artifact_name(self)
    }

    pub fn version_output_dir(&self) -> PathBuf {
        let dir = self.output_dir.join(&self.app_version);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).ok();
        }
        dir
    }

    pub fn output_file(&self) -> PathBuf {
        self.version_output_dir().join(self.output_file_name())
    }

    pub fn packaging_dir(&self) -> PathBuf {
        let stem = if self.package_format.is_empty() {
            format!("{}_direct", self.output_file_name())
        } else {
            self.output_file_name().replace(
                &format!(".{}", self.package_format),
                &format!("_{}", self.package_format),
            )
        };
        let dir = self.version_output_dir().join(stem);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).ok();
        }
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    pub fn first_build_output_file(&self) -> Option<&Path> {
        self.build_output_files.first().map(|p| p.as_path())
    }
}

#[derive(Debug)]
pub struct PackageResult {
    pub artifacts: Vec<PathBuf>,
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("{0}")]
    General(String),
    #[error("Missing tool: {0}")]
    MissingTool(String),
    #[error("Command '{command}' failed: {stderr}")]
    CommandFailed { command: String, stderr: String },
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait AppPackager {
    fn name(&self) -> &str;
    fn platform(&self) -> Platform;
    fn package_format(&self) -> &str;

    fn is_supported_on_current_platform(&self) -> bool {
        true
    }

    fn matches(&self, platform: &Platform, target: Option<&str>) -> bool {
        self.platform() == *platform && target.is_none_or(|t| self.name() == t)
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> PackageConfig {
        PackageConfig {
            app_name: "hello_world".to_string(),
            app_binary_name: "hello_world".to_string(),
            app_version: "1.2.3+4".to_string(),
            build_mode: "release".to_string(),
            platform: Platform::Linux,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "deb".to_string(),
            is_installer: true,
            build_output_dir: PathBuf::new(),
            build_output_files: vec![],
            output_dir: PathBuf::new(),
        }
    }

    #[test]
    fn default_artifact_name() {
        assert_eq!(
            base_config().output_file_name(),
            "hello_world-1.2.3+4-linux-setup.deb"
        );
    }

    #[test]
    fn flavor_artifact_name() {
        let mut config = base_config();
        config.flavor = Some("dev".to_string());
        assert_eq!(
            config.output_file_name(),
            "hello_world-dev-1.2.3+4-linux-setup.deb"
        );
    }

    #[test]
    fn channel_artifact_name_replaces_flavor_segment() {
        let mut config = base_config();
        config.flavor = Some("dev".to_string());
        config.channel = Some("beta".to_string());
        assert_eq!(
            config.output_file_name(),
            "hello_world-beta-1.2.3+4-linux-setup.deb"
        );
    }

    #[test]
    fn artifact_name_template_variables() {
        let mut config = base_config();
        config.artifact_name = Some("{{name}}_{{build_name}}_amd64.{{ext}}".to_string());
        assert_eq!(config.output_file_name(), "hello_world_1.2.3_amd64.deb");
    }

    #[test]
    fn artifact_name_template_sections() {
        let mut config = base_config();
        config.artifact_name = Some(
            "{{name}}{{#flavor}}-{{flavor}}{{/flavor}}-{{build_name}}{{#has_build_number}}+{{build_number}}{{/has_build_number}}{{#is_profile}}-{{build_mode}}{{/is_profile}}-{{platform}}{{#is_installer}}-setup{{/is_installer}}{{#ext}}.{{ext}}{{/ext}}"
                .to_string(),
        );
        // Matches Dart's default template output exactly
        assert_eq!(
            config.output_file_name(),
            "hello_world-1.2.3+4-linux-setup.deb"
        );
    }

    #[test]
    fn artifact_name_literal_passthrough() {
        let mut config = base_config();
        config.artifact_name = Some("MyApp-Installer.deb".to_string());
        assert_eq!(config.output_file_name(), "MyApp-Installer.deb");
    }
}
