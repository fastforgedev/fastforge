//! Helpers shared by all publishers.

use fastforge_core::{PublishConfig, PublishError, PublishProgressCallback};
use reqwest::blocking::multipart::Part;
use reqwest::blocking::{Body, Client};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP client used by every publisher.
///
/// `reqwest::blocking::Client::new()` applies a 30s *total* timeout, which
/// aborts long uploads. Dart's Dio has no timeout at all, so only connection
/// establishment is bounded here.
pub(crate) fn http_client() -> Result<Client, PublishError> {
    Client::builder()
        .timeout(None)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .map_err(to_publish_error)
}

pub(crate) fn to_publish_error(error: impl std::fmt::Display) -> PublishError {
    PublishError::General(error.to_string())
}

/// Dart's wording for a missing environment variable.
pub(crate) fn missing_env(key: &str) -> PublishError {
    PublishError::General(format!("Missing `{key}` environment variable."))
}

/// Reads a required variable through [`PublishConfig::env_var`].
pub(crate) fn required_env(config: &PublishConfig, key: &str) -> Result<String, PublishError> {
    config.env_var(key).ok_or_else(|| missing_env(key))
}

pub(crate) fn artifact_path(config: &PublishConfig) -> Result<&str, PublishError> {
    config
        .artifact_path
        .as_deref()
        .ok_or_else(|| PublishError::MissingArgument("artifact_path".to_string()))
}

pub(crate) fn file_name(path: &str) -> Result<String, PublishError> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            PublishError::General(format!("Cannot infer file name from artifact path: {path}"))
        })
}

/// Returns the first non-empty publish argument among `keys`. Empty values
/// count as missing, like Dart's `isEmpty` checks.
pub(crate) fn argument(config: &PublishConfig, keys: &[&str]) -> Option<String> {
    let arguments = config.publish_arguments.as_ref()?;
    keys.iter()
        .find_map(|key| arguments.get(*key).filter(|value| !value.is_empty()))
        .cloned()
}

/// First non-empty publish argument among `argument_keys`, falling back to
/// the first non-empty variable among `env_keys`.
pub(crate) fn argument_or_env(
    config: &PublishConfig,
    argument_keys: &[&str],
    env_keys: &[&str],
) -> Option<String> {
    argument(config, argument_keys).or_else(|| env_keys.iter().find_map(|key| config.env_var(key)))
}

// ── App version ───────────────────────────────────────────────────────────────

/// A parsed app version (`major.minor.patch[-pre][+build]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppVersion {
    /// The version as written (Dart's `Version.toString()`).
    pub text: String,
    /// Version core, without pre-release and build metadata.
    pub core: String,
    /// Build metadata (`Version.build.join('.')`), empty when absent.
    pub build: String,
}

impl AppVersion {
    pub fn parse(text: &str) -> Result<Self, PublishError> {
        let version = fastforge_core::AppVersion::parse(text).map_err(PublishError::General)?;
        Ok(Self {
            text: version.text,
            core: version.core,
            build: version.build_number.unwrap_or_default(),
        })
    }
}

/// The app version to publish: `config.app_version`, then the `app-version`
/// publish argument, then the `version` of `pubspec.yaml` in the current
/// directory (Dart's `PublishConfig` fallback).
pub(crate) fn resolve_app_version(
    config: &PublishConfig,
) -> Result<Option<AppVersion>, PublishError> {
    let version = match config
        .app_version
        .clone()
        .or_else(|| argument(config, &["app-version"]))
    {
        Some(version) => Some(version),
        None => pubspec_version(Path::new("pubspec.yaml"))?,
    };
    version
        .map(|version| AppVersion::parse(&version))
        .transpose()
}

fn pubspec_version(path: &Path) -> Result<Option<String>, PublishError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let pubspec: serde_yaml::Value = serde_yaml::from_str(&content).map_err(to_publish_error)?;
    match pubspec.get("version") {
        None => Ok(None),
        Some(serde_yaml::Value::String(version)) => Ok(Some(version.clone())),
        Some(_) => Err(PublishError::General(
            "pubspec.yaml version must be a string".into(),
        )),
    }
}

// ── Uploads ───────────────────────────────────────────────────────────────────

/// Wraps a reader and reports how many bytes have been read so far.
pub(crate) struct UploadProgressReader<R> {
    inner: R,
    sent: u64,
    total: u64,
    on_progress: Option<PublishProgressCallback>,
}

impl<R: Read> UploadProgressReader<R> {
    pub fn new(inner: R, total: u64, on_progress: Option<PublishProgressCallback>) -> Self {
        if let Some(callback) = &on_progress {
            callback(0, total);
        }
        Self {
            inner,
            sent: 0,
            total,
            on_progress,
        }
    }
}

impl<R: Read> Read for UploadProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.inner.read(buf)?;
        if bytes_read > 0 {
            self.sent += bytes_read as u64;
            if let Some(callback) = &self.on_progress {
                callback(self.sent, self.total);
            }
        }
        Ok(bytes_read)
    }
}

/// Opens `path` as a sized request body that reports upload progress.
pub(crate) fn file_body(
    path: &str,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<Body, PublishError> {
    let file = File::open(path).map_err(to_publish_error)?;
    let total = file.metadata().map_err(to_publish_error)?.len();
    let reader = UploadProgressReader::new(file, total, on_progress.cloned());
    Ok(Body::sized(reader, total))
}

/// Multipart file part with a known length, so the form is sent with a
/// `Content-Length` header like Dart's `MultipartFile.fromFile`.
pub(crate) fn file_part(
    path: &str,
    file_name: &str,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<Part, PublishError> {
    let file = File::open(path).map_err(to_publish_error)?;
    let total = file.metadata().map_err(to_publish_error)?.len();
    let reader = UploadProgressReader::new(file, total, on_progress.cloned());
    Part::reader_with_length(reader, total)
        .file_name(file_name.to_string())
        .mime_str("application/octet-stream")
        .map_err(to_publish_error)
}

// ── External commands ─────────────────────────────────────────────────────────

pub(crate) struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    /// Exit code for error messages (`-1` when killed by a signal).
    pub fn code(&self) -> i32 {
        self.status.code().unwrap_or(-1)
    }
}

/// Runs `command`, echoing its stdout/stderr live (like Dart's shell
/// executor) while capturing both for parsing.
pub(crate) fn run_streaming(command: &mut Command) -> io::Result<CommandOutput> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout_thread = child
        .stdout
        .take()
        .map(|pipe| thread::spawn(move || tee(pipe, io::stdout())));
    let stderr_thread = child
        .stderr
        .take()
        .map(|pipe| thread::spawn(move || tee(pipe, io::stderr())));
    let status = child.wait()?;
    let collect = |handle: Option<thread::JoinHandle<Vec<u8>>>| {
        handle
            .and_then(|handle| handle.join().ok())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    };
    Ok(CommandOutput {
        status,
        stdout: collect(stdout_thread),
        stderr: collect(stderr_thread),
    })
}

fn tee(mut reader: impl Read, mut writer: impl Write) -> Vec<u8> {
    let mut captured = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let _ = writer.write_all(&buffer[..n]);
                let _ = writer.flush();
                captured.extend_from_slice(&buffer[..n]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    captured
}

/// Finds the first `http(s)://` URL right after `marker` in `log`, mirroring
/// Dart's `(?<=<marker>)\bhttps?:\/\/\S+\b`: the URL runs to the next
/// whitespace and trailing non-word characters are dropped.
pub(crate) fn extract_url_after(log: &str, marker: &str) -> Option<String> {
    log.match_indices(marker).find_map(|(index, _)| {
        let rest = &log[index + marker.len()..];
        if !(rest.starts_with("http://") || rest.starts_with("https://")) {
            return None;
        }
        let candidate = rest.split(char::is_whitespace).next().unwrap_or_default();
        let url = candidate.trim_end_matches(|c: char| !(c.is_alphanumeric() || c == '_'));
        let scheme_len = if url.starts_with("https://") { 8 } else { 7 };
        (url.len() > scheme_len).then(|| url.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_invalid_version_is_an_error_instead_of_pubspec_fallback() {
        let config = PublishConfig {
            app_version: Some("1.2.3+4+5".into()),
            ..Default::default()
        };
        assert!(
            resolve_app_version(&config)
                .unwrap_err()
                .to_string()
                .contains("Invalid app version")
        );
        let config = PublishConfig {
            publish_arguments: Some([("app-version".into(), "1.2".into())].into()),
            ..Default::default()
        };
        assert!(resolve_app_version(&config).is_err());
    }

    #[test]
    fn app_version_parses_core_and_build() {
        let version = AppVersion::parse("1.2.3+45").unwrap();
        assert_eq!(version.text, "1.2.3+45");
        assert_eq!(version.core, "1.2.3");
        assert_eq!(version.build, "45");

        let version = AppVersion::parse("2.0.0-beta.1").unwrap();
        assert_eq!(version.core, "2.0.0");
        assert_eq!(version.build, "");

        let version = AppVersion::parse("2.0.0-beta.1+7.8").unwrap();
        assert_eq!(version.core, "2.0.0");
        assert_eq!(version.build, "7.8");
    }

    #[test]
    fn argument_skips_empty_values() {
        let config = PublishConfig {
            publish_arguments: Some(
                [
                    ("a".to_string(), String::new()),
                    ("b".to_string(), "value".to_string()),
                ]
                .into(),
            ),
            ..Default::default()
        };
        assert_eq!(argument(&config, &["a"]), None);
        assert_eq!(argument(&config, &["a", "b"]), Some("value".to_string()));
    }

    #[test]
    fn argument_or_env_falls_back_to_environment_when_empty() {
        let config = PublishConfig {
            publish_arguments: Some([("endpoint".to_string(), String::new())].into()),
            environment: [(
                "FASTFORGE_TEST_ENDPOINT".to_string(),
                "play.min.io".to_string(),
            )]
            .into(),
            ..Default::default()
        };
        assert_eq!(
            argument_or_env(&config, &["endpoint"], &["FASTFORGE_TEST_ENDPOINT"]),
            Some("play.min.io".to_string())
        );
    }

    #[test]
    fn extract_url_after_matches_dart_regex() {
        let log = "Deploying...\nProduction: https://my-app.vercel.app [2s]\n";
        assert_eq!(
            extract_url_after(log, "Production: "),
            Some("https://my-app.vercel.app".to_string())
        );
        let log = "✔  Hosting URL: https://demo.web.app/\n";
        assert_eq!(
            extract_url_after(log, "Hosting URL: "),
            Some("https://demo.web.app".to_string())
        );
        assert_eq!(
            extract_url_after("Production: pending", "Production: "),
            None
        );
        assert_eq!(extract_url_after("nothing here", "Production: "), None);
    }
}
