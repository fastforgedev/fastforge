mod custom;
mod flutter;
mod gradle;
mod xcode;

pub use crate::flutter::command::FlutterCommand;
use crate::flutter::{
    AndroidAabBuilder, AndroidApkBuilder, IOSBuilder, LinuxBuilder, MacOSBuilder, OhosAppBuilder,
    OhosHapBuilder, WebBuilder, WindowsBuilder,
};
pub use crate::flutter::{FlutterVersion, PubspecInfo};
pub use fastforge_core::AppBuilder;
pub use fastforge_core::{BuildConfig, BuildError, BuildMode, BuildRequest, BuildResult, Platform};
use serde_json::{Map, Value};
use std::time::Instant;

pub use crate::custom::{CustomAppBuilder, CustomBuilder};
pub use crate::gradle::{
    GradleAndroidAabBuilder, GradleAndroidApkBuilder, GradleAppBuilder, GradleKmpAndroidAabBuilder,
    GradleKmpAndroidApkBuilder, GradleKmpDesktopBuilder, GradleKmpIosFrameworkBuilder,
};
pub use crate::xcode::ios::{IOSXcodeAppBuilder, IOSXcodeBuilder};
pub use crate::xcode::{MacOSXcodeAppBuilder, MacOSXcodeBuilder};

pub struct FlutterAppBuilder {
    builders: Vec<Box<dyn AppBuilder + Send + Sync>>,
}

impl Default for FlutterAppBuilder {
    fn default() -> Self {
        Self {
            builders: vec![
                Box::new(AndroidAabBuilder),
                Box::new(AndroidApkBuilder),
                Box::new(IOSBuilder),
                Box::new(LinuxBuilder),
                Box::new(MacOSBuilder),
                Box::new(OhosHapBuilder),
                Box::new(OhosAppBuilder),
                Box::new(WebBuilder),
                Box::new(WindowsBuilder),
            ],
        }
    }
}

impl FlutterAppBuilder {
    pub fn clean(
        &self,
        environment: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<(), BuildError> {
        FlutterCommand::new(environment).clean()
    }

    /// Whether the builder for `(platform, target)` can run on this host.
    /// `None` when no builder matches.
    pub fn is_supported_on_current_platform(
        &self,
        platform: &Platform,
        target: Option<&str>,
    ) -> Option<bool> {
        self.builders
            .iter()
            .find(|b| b.matches(platform, target))
            .map(|b| b.is_supported_on_current_platform())
    }

    pub fn build(
        &self,
        platform: &Platform,
        target: Option<&str>,
        arguments: Map<String, Value>,
        environment: Option<std::collections::HashMap<String, String>>,
    ) -> Result<BuildResult, BuildError> {
        let builder = self
            .builders
            .iter()
            .find(|b| b.matches(platform, target))
            .ok_or_else(|| {
                BuildError::UnsupportedBuilder(format!(
                    "No builder found for platform={} target={}",
                    platform.as_str(),
                    target.unwrap_or("")
                ))
            })?;

        if !builder.is_supported_on_current_platform() {
            return Err(BuildError::UnsupportedPlatform(format!(
                "{} is not supported on the current platform",
                builder.name()
            )));
        }

        let config = BuildConfig::new(arguments);
        builder.validate_arguments(&config)?;

        let mut build_arguments = encode_build_arguments(&config.arguments);
        // Like Dart's `AppBuilder.build`, default `--build-name` /
        // `--build-number` to the pubspec version unless given explicitly.
        if let Some(pubspec) = PubspecInfo::load("pubspec.yaml") {
            if !config.arguments.contains_key("build-name") {
                build_arguments.extend(["--build-name".to_string(), pubspec.build_name]);
            }
            if !config.arguments.contains_key("build-number") {
                build_arguments.extend(["--build-number".to_string(), pubspec.build_number]);
            }
        }

        let start = Instant::now();
        let flutter = FlutterCommand::new(environment.as_ref());
        let (exit, stderr) =
            flutter.build_with_echo(builder.build_subcommand(), &build_arguments)?;
        if exit != 0 {
            let stderr = stderr.trim();
            return Err(BuildError::CommandFailed(if stderr.is_empty() {
                format!("flutter build failed with exit code {}", exit)
            } else {
                stderr.to_string()
            }));
        }

        let (output_directory, output_files) =
            builder.resolve_output_files(&config, environment.as_ref())?;

        // Directory-style outputs (linux/windows/web) report no files, as in
        // Dart; for those only the output directory has to exist.
        if !builder.outputs_directory() && output_files.is_empty() {
            return Err(BuildError::ArtifactNotFound(format!(
                "No build artifacts found in {}",
                output_directory.display()
            )));
        }
        if builder.outputs_directory() && !output_directory.is_dir() {
            return Err(BuildError::ArtifactNotFound(format!(
                "Build output directory not found: {}",
                output_directory.display()
            )));
        }

        Ok(builder.build_result(
            config,
            output_directory,
            output_files,
            start.elapsed().as_millis(),
        ))
    }
}

pub fn build(request: BuildRequest) -> Result<BuildResult, BuildError> {
    FlutterAppBuilder::default().build(
        &request.platform,
        request.target.as_deref(),
        request.arguments,
        request.environment,
    )
}

fn encode_build_arguments(arguments: &Map<String, Value>) -> Vec<String> {
    let mut output = Vec::new();

    for (key, value) in arguments {
        match value {
            Value::Null | Value::Bool(_) => {
                output.push(format!("--{}", key));
            }
            Value::Array(items) => {
                for item in items {
                    output.push(format!("--{}", key));
                    output.push(value_to_cli_string(item));
                }
            }
            Value::Object(map) => {
                for (sub_key, sub_value) in map {
                    output.push(format!("--{}", key));
                    output.push(format!("{}={}", sub_key, value_to_cli_string(sub_value)));
                }
            }
            _ => {
                output.push(format!("--{}", key));
                output.push(value_to_cli_string(value));
            }
        }
    }

    output
}

fn value_to_cli_string(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        s.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encode_arguments_matches_dart_behavior() {
        let mut args = Map::new();
        args.insert("verbose".to_string(), Value::Bool(true));
        args.insert("flavor".to_string(), Value::String("dev".to_string()));
        args.insert("build-number".to_string(), Value::Number(42.into()));
        args.insert(
            "dart-define".to_string(),
            json!({"APP_ENV":"dev","FOO":"bar"}),
        );
        args.insert("null-flag".to_string(), Value::Null);

        let actual = encode_build_arguments(&args);
        let expected = vec![
            "--verbose",
            "--flavor",
            "dev",
            "--build-number",
            "42",
            "--dart-define",
            "APP_ENV=dev",
            "--dart-define",
            "FOO=bar",
            "--null-flag",
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn build_config_mode_and_flavor() {
        let mut args = Map::new();
        args.insert("profile".to_string(), Value::Bool(true));
        args.insert("flavor".to_string(), Value::String("prod".to_string()));
        let config = BuildConfig::new(args);
        assert_eq!(config.mode(), BuildMode::Profile);
        assert_eq!(config.flavor(), Some("prod"));
    }
}
