use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

// ── Types ─────────────────────────────────────────────────────────────────────

pub type PublishProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync + 'static>;

#[derive(Debug, Clone, Default)]
pub struct PublishConfig {
    pub app_version: Option<String>,
    pub artifact_path: Option<String>,
    pub publish_arguments: Option<HashMap<String, String>>,
    /// Process environment plus `distribute_options.yaml` variables
    /// (global/release/job), as Dart passes `environment` to publishers.
    /// Lookups fall back to the process environment.
    pub environment: HashMap<String, String>,
}

impl PublishConfig {
    /// Looks up a variable in `environment`, then the process environment.
    /// Empty values count as unset (Dart checks `isEmpty`).
    pub fn env_var(&self, key: &str) -> Option<String> {
        self.environment
            .get(key)
            .cloned()
            .filter(|v| !v.is_empty())
            .or_else(|| std::env::var(key).ok().filter(|v| !v.is_empty()))
    }
}

#[derive(Debug)]
pub struct PublishResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum PublishError {
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),
    #[error("Missing publish argument '{0}'")]
    MissingArgument(String),
    #[error("HTTP request failed: {0}")]
    HttpError(String),
    #[error("API error: {status} {message}")]
    ApiError { status: String, message: String },
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    General(String),
}

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait AppPublisher {
    fn new() -> Self;
    fn name(&self) -> &str;
    fn is_supported_on_current_platform(&self) -> bool;

    fn perform_publish(
        &self,
        config: &PublishConfig,
        on_progress: Option<&PublishProgressCallback>,
    ) -> Result<PublishResult, PublishError>;

    fn publish(
        &self,
        config: PublishConfig,
        on_progress: Option<PublishProgressCallback>,
    ) -> Result<PublishResult, PublishError> {
        if let Some(version) = config.app_version.as_deref().or_else(|| {
            config
                .publish_arguments
                .as_ref()
                .and_then(|args| args.get("app-version").map(String::as_str))
        }) {
            crate::AppVersion::parse(version).map_err(PublishError::General)?;
        }
        self.perform_publish(&config, on_progress.as_ref())
    }
}
