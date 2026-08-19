use anyhow::{Result, anyhow};
use clap::Args;
use fastforge_app_publisher::{
    AppGalleryPublisher, AppPublisher, AppStorePublisher, CosPublisher, CustomPublisher,
    FirPublisher, FirebaseHostingPublisher, FirebasePublisher, GitHubPublisher, OssPublisher,
    PgyerPublisher, PlayStorePublisher, PublishConfig, PublishProgressCallback, QiniuPublisher,
    S3Publisher, VercelPublisher,
};
use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::sync::Arc;

#[derive(Args)]
pub struct PublishArgs {
    #[arg(long = "path")]
    pub path: Option<String>,
    /// Comma-separated list of target providers to publish to.
    #[arg(short, long = "targets", alias = "target", value_name = "TARGET,...")]
    pub targets: Option<String>,
    /// The version of the app (semantic versioning, e.g. 1.0.0, 2.1.3-beta.1).
    #[arg(long = "app-version")]
    pub app_version: Option<String>,
    #[arg(long = "publish-arg", value_name = "KEY=VALUE")]
    pub publish_args: Vec<String>,
}

pub async fn execute(args: &PublishArgs) -> Result<()> {
    log::info!("Executing publish command");

    let artifact_path = args
        .path
        .clone()
        .ok_or_else(|| anyhow!("The 'path' option is mandatory!"))?;

    let targets: Vec<String> = args
        .targets
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if targets.is_empty() {
        return Err(anyhow!("At least one 'target' must be specified!"));
    }

    let mut publish_arguments = parse_publish_args(&args.publish_args)?;
    if let Some(version) = &args.app_version {
        publish_arguments
            .entry("app-version".to_string())
            .or_insert_with(|| version.clone());
    }

    for target in &targets {
        let message = publish_artifact(&artifact_path, target, publish_arguments.clone())?;
        println!("{}", message);
    }
    Ok(())
}

/// Accepts Dart-style `<target>-` prefixed publish arguments (e.g.
/// `github-repo`, `minio-bucket`): every `<target>-*` key is also made
/// available without its prefix, mirroring Dart's
/// `UnifiedDistributor.publish` prefix stripping. Unprefixed keys are kept
/// as-is, so both conventions work.
fn resolve_target_arguments(
    target: &str,
    publish_arguments: HashMap<String, String>,
) -> HashMap<String, String> {
    let prefix = format!("{}-", target);
    let mut resolved = publish_arguments.clone();
    for (key, value) in &publish_arguments {
        if let Some(stripped) = key.strip_prefix(&prefix) {
            resolved
                .entry(stripped.to_string())
                .or_insert_with(|| value.clone());
        }
    }
    resolved
}

/// Renders upload progress to stderr while publishing (mirrors Dart's
/// `ProgressBar`). Disabled when stderr is not a terminal.
fn progress_callback(target: &str) -> Option<PublishProgressCallback> {
    if !std::io::stderr().is_terminal() {
        return None;
    }
    let target = target.to_string();
    Some(Arc::new(move |sent: u64, total: u64| {
        if total == 0 {
            return;
        }
        let percentage = sent * 100 / total;
        let filled = (percentage / 5) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(20 - filled));
        let mut stderr = std::io::stderr();
        let _ = write!(
            stderr,
            "\rPublishing to {}: {} {}/{} {}%",
            target, bar, sent, total, percentage
        );
        if sent >= total {
            let _ = writeln!(stderr);
        }
        let _ = stderr.flush();
    }))
}

pub fn publish_artifact(
    artifact_path: &str,
    target: &str,
    publish_arguments: HashMap<String, String>,
) -> Result<String> {
    let target = target.to_ascii_lowercase();
    let publish_arguments = resolve_target_arguments(&target, publish_arguments);
    let app_version = publish_arguments.get("app-version").cloned();
    let publish_config = PublishConfig {
        app_version,
        artifact_path: Some(artifact_path.to_string()),
        publish_arguments: if publish_arguments.is_empty() {
            None
        } else {
            Some(publish_arguments)
        },
    };

    let progress = progress_callback(&target);
    let result = match target.as_str() {
        "s3" | "minio" => S3Publisher::new().publish(publish_config, progress),
        "qiniu" => QiniuPublisher::new().publish(publish_config, progress),
        "oss" => OssPublisher::new().publish(publish_config, progress),
        "cos" => CosPublisher::new().publish(publish_config, progress),
        "fir" => FirPublisher::new().publish(publish_config, progress),
        "firebase" => FirebasePublisher::new().publish(publish_config, progress),
        "firebase-hosting" => FirebaseHostingPublisher::new().publish(publish_config, progress),
        "github" => GitHubPublisher::new().publish(publish_config, progress),
        "appstore" => AppStorePublisher::new().publish(publish_config, progress),
        "appgallery" => AppGalleryPublisher::new().publish(publish_config, progress),
        "playstore" => PlayStorePublisher::new().publish(publish_config, progress),
        "pgyer" => PgyerPublisher::new().publish(publish_config, progress),
        "vercel" => VercelPublisher::new().publish(publish_config, progress),
        "custom" => CustomPublisher::new().publish(publish_config, progress),
        _ => {
            return Err(anyhow!(
                "Unsupported publish target: `{}`. Currently supported: s3, minio, qiniu, oss, cos, fir, firebase, firebase-hosting, github, appstore, appgallery, playstore, pgyer, vercel, custom",
                target
            ));
        }
    }
    .map_err(|e| anyhow!(e.to_string()))?;

    Ok(result.message)
}

fn parse_publish_args(items: &[String]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();

    for item in items {
        let (key, value) = item
            .split_once('=')
            .ok_or_else(|| anyhow!("Invalid --publish-arg item: `{item}`; expected KEY=VALUE"))?;
        let key = key.trim();
        if key.is_empty() {
            return Err(anyhow!(
                "Invalid --publish-arg item: `{item}`; key cannot be empty"
            ));
        }
        map.insert(key.to_string(), value.to_string());
    }

    Ok(map)
}
