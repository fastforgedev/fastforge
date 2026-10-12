use anyhow::{Result, anyhow};
use clap::Args;
use fastforge_app_publisher::{
    AppGalleryPublisher, AppPublisher, AppStorePublisher, AurPublisher, CosPublisher,
    CustomPublisher, FirPublisher, FirebaseHostingPublisher, FirebasePublisher, GitHubPublisher,
    MinioPublisher, OssPublisher, PgyerPublisher, PlayStorePublisher, PublishConfig,
    PublishProgressCallback, QiniuPublisher, S3Publisher, VercelPublisher,
};
use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::sync::Arc;

use crate::config::DistributeOptions;
use crate::utils::{bright_green, global_variables};

/// Publish the built application artifacts to distribution platforms.
///
/// Besides the provider-specific options below (the same flags as the Dart
/// CLI), any publisher argument can be passed with `--publish-arg KEY=VALUE`.
#[derive(Args)]
pub struct PublishArgs {
    /// The path to the application bundle to publish.
    #[arg(long = "path")]
    pub path: Option<String>,
    /// The target provider(s) to publish to.
    #[arg(
        short,
        long = "targets",
        alias = "target",
        value_name = "appgallery,appstore,fir,firebase,github,playstore,pgyer,qiniu,vercel"
    )]
    pub targets: Option<String>,
    /// The version of the app
    /// Must follow semantic versioning format, e.g., 1.0.0, 2.1.3-beta.1
    #[arg(long = "app-version")]
    pub app_version: Option<String>,
    /// Extra publisher argument; may be repeated.
    #[arg(long = "publish-arg", value_name = "KEY=VALUE")]
    pub publish_args: Vec<String>,

    #[command(flatten)]
    pub provider: ProviderArgs,
}

/// Provider-specific options of the Dart CLI (`command_publish.dart`). They
/// are forwarded as `<target>-<name>` publish arguments; the prefix is
/// stripped for the matching target.
#[derive(Args, Default)]
pub struct ProviderArgs {
    /// The unique ID of the application on AppGallery.
    #[arg(long = "appgallery-app-id", help_heading = "appgallery")]
    pub appgallery_app_id: Option<String>,

    /// The unique ID of the application on Firebase.
    /// This is NOT your bundle identifier
    #[arg(long = "firebase-app", help_heading = "firebase")]
    pub firebase_app: Option<String>,
    /// The release notes for the published application.
    #[arg(long = "firebase-release-notes", help_heading = "firebase")]
    pub firebase_release_notes: Option<String>,
    /// The path of a file containing the release notes
    /// This is a more extensive alternative to firebase-release-notes
    #[arg(long = "firebase-release-notes-file", help_heading = "firebase")]
    pub firebase_release_notes_file: Option<String>,
    /// The testers that will be notified about the published application.
    #[arg(long = "firebase-testers", help_heading = "firebase")]
    pub firebase_testers: Option<String>,
    /// The path of a file containing testers that will be notified
    /// This is a more extensive alternative to firebase-testers
    #[arg(long = "firebase-testers-file", help_heading = "firebase")]
    pub firebase_testers_file: Option<String>,
    /// The groups that will be notified about the published application.
    #[arg(long = "firebase-groups", help_heading = "firebase")]
    pub firebase_groups: Option<String>,
    /// The path of a file containing groups that will be notified
    /// This is a more extensive alternative to firebase-groups
    #[arg(long = "firebase-groups-file", help_heading = "firebase")]
    pub firebase_groups_file: Option<String>,

    #[arg(
        long = "firebase-hosting-project-id",
        help_heading = "firebase-hosting"
    )]
    pub firebase_hosting_project_id: Option<String>,

    /// The repository to publish to, format: <owner>/<repo>
    #[arg(long = "github-repo", help_heading = "github")]
    pub github_repo: Option<String>,
    /// [Deprecated] The name of the target GitHub repository owner (namespace)
    #[arg(long = "github-repo-owner", help_heading = "github")]
    pub github_repo_owner: Option<String>,
    /// [Deprecated] The name of the target GitHub repository
    #[arg(long = "github-repo-name", help_heading = "github")]
    pub github_repo_name: Option<String>,
    /// The title of the new release on GitHub
    #[arg(long = "github-release-title", help_heading = "github")]
    pub github_release_title: Option<String>,
    /// Whether to create a draft release
    #[arg(
        long = "github-release-draft",
        value_name = "true|false",
        default_value = "false",
        help_heading = "github"
    )]
    pub github_release_draft: String,
    /// Whether to create a prerelease
    #[arg(
        long = "github-release-prerelease",
        value_name = "true|false",
        default_value = "false",
        help_heading = "github"
    )]
    pub github_release_prerelease: String,

    #[arg(long = "minio-endpoint", help_heading = "minio")]
    pub minio_endpoint: Option<String>,
    #[arg(long = "minio-access-key", help_heading = "minio")]
    pub minio_access_key: Option<String>,
    #[arg(long = "minio-secret-key", help_heading = "minio")]
    pub minio_secret_key: Option<String>,
    #[arg(long = "minio-region", help_heading = "minio")]
    pub minio_region: Option<String>,
    #[arg(long = "minio-bucket", help_heading = "minio")]
    pub minio_bucket: Option<String>,
    #[arg(long = "minio-savekey-prefix", help_heading = "minio")]
    pub minio_savekey_prefix: Option<String>,

    /// Upload acceleration: 1=overseas, 2=domestic, empty=auto-detect
    #[arg(long = "pgyer-oversea", value_name = "1|2", help_heading = "pgyer")]
    pub pgyer_oversea: Option<String>,
    /// Installation type: 1=public, 2=password, 3=invite (default: 1)
    #[arg(
        long = "pgyer-install-type",
        value_name = "1|2|3",
        help_heading = "pgyer"
    )]
    pub pgyer_install_type: Option<String>,
    /// App installation password (required for password installation)
    #[arg(long = "pgyer-password", help_heading = "pgyer")]
    pub pgyer_password: Option<String>,
    /// Application description (optional)
    #[arg(long = "pgyer-description", help_heading = "pgyer")]
    pub pgyer_description: Option<String>,
    /// Version update description (optional)
    #[arg(long = "pgyer-update-description", help_heading = "pgyer")]
    pub pgyer_update_description: Option<String>,
    /// Installation validity: 1=set time, 2=permanent (optional)
    #[arg(
        long = "pgyer-install-date",
        value_name = "1|2",
        help_heading = "pgyer"
    )]
    pub pgyer_install_date: Option<String>,
    /// Installation validity start date (e.g., 2018-01-01)
    #[arg(
        long = "pgyer-install-start-date",
        value_name = "YYYY-MM-DD",
        help_heading = "pgyer"
    )]
    pub pgyer_install_start_date: Option<String>,
    /// Installation validity end date (e.g., 2018-12-31)
    #[arg(
        long = "pgyer-install-end-date",
        value_name = "YYYY-MM-DD",
        help_heading = "pgyer"
    )]
    pub pgyer_install_end_date: Option<String>,
    /// Channel shortcut for update (e.g., abcd)
    #[arg(long = "pgyer-channel-shortcut", help_heading = "pgyer")]
    pub pgyer_channel_shortcut: Option<String>,

    #[arg(long = "playstore-package-name", help_heading = "playstore")]
    pub playstore_package_name: Option<String>,
    #[arg(long = "playstore-track", help_heading = "playstore")]
    pub playstore_track: Option<String>,

    #[arg(long = "qiniu-bucket", help_heading = "qiniu")]
    pub qiniu_bucket: Option<String>,
    #[arg(long = "qiniu-bucket-domain", help_heading = "qiniu")]
    pub qiniu_bucket_domain: Option<String>,
    #[arg(long = "qiniu-savekey-prefix", help_heading = "qiniu")]
    pub qiniu_savekey_prefix: Option<String>,

    #[arg(long = "vercel-org-id", help_heading = "vercel")]
    pub vercel_org_id: Option<String>,
    #[arg(long = "vercel-project-id", help_heading = "vercel")]
    pub vercel_project_id: Option<String>,
}

impl ProviderArgs {
    /// The provider options as `<target>-<name>` publish arguments (Dart's
    /// `publishArguments` map, minus unset values).
    fn to_arguments(&self) -> HashMap<String, String> {
        let entries = [
            ("appgallery-app-id", self.appgallery_app_id.as_ref()),
            ("firebase-app", self.firebase_app.as_ref()),
            (
                "firebase-release-notes",
                self.firebase_release_notes.as_ref(),
            ),
            (
                "firebase-release-notes-file",
                self.firebase_release_notes_file.as_ref(),
            ),
            ("firebase-testers", self.firebase_testers.as_ref()),
            ("firebase-testers-file", self.firebase_testers_file.as_ref()),
            ("firebase-groups", self.firebase_groups.as_ref()),
            ("firebase-groups-file", self.firebase_groups_file.as_ref()),
            (
                "firebase-hosting-project-id",
                self.firebase_hosting_project_id.as_ref(),
            ),
            ("github-repo", self.github_repo.as_ref()),
            ("github-repo-owner", self.github_repo_owner.as_ref()),
            ("github-repo-name", self.github_repo_name.as_ref()),
            ("github-release-title", self.github_release_title.as_ref()),
            ("github-release-draft", Some(&self.github_release_draft)),
            (
                "github-release-prerelease",
                Some(&self.github_release_prerelease),
            ),
            ("minio-endpoint", self.minio_endpoint.as_ref()),
            ("minio-access-key", self.minio_access_key.as_ref()),
            ("minio-secret-key", self.minio_secret_key.as_ref()),
            ("minio-region", self.minio_region.as_ref()),
            ("minio-bucket", self.minio_bucket.as_ref()),
            ("minio-savekey-prefix", self.minio_savekey_prefix.as_ref()),
            ("pgyer-oversea", self.pgyer_oversea.as_ref()),
            ("pgyer-install-type", self.pgyer_install_type.as_ref()),
            ("pgyer-password", self.pgyer_password.as_ref()),
            ("pgyer-description", self.pgyer_description.as_ref()),
            (
                "pgyer-update-description",
                self.pgyer_update_description.as_ref(),
            ),
            ("pgyer-install-date", self.pgyer_install_date.as_ref()),
            (
                "pgyer-install-start-date",
                self.pgyer_install_start_date.as_ref(),
            ),
            (
                "pgyer-install-end-date",
                self.pgyer_install_end_date.as_ref(),
            ),
            (
                "pgyer-channel-shortcut",
                self.pgyer_channel_shortcut.as_ref(),
            ),
            (
                "playstore-package-name",
                self.playstore_package_name.as_ref(),
            ),
            ("playstore-track", self.playstore_track.as_ref()),
            ("qiniu-bucket", self.qiniu_bucket.as_ref()),
            ("qiniu-bucket-domain", self.qiniu_bucket_domain.as_ref()),
            ("qiniu-savekey-prefix", self.qiniu_savekey_prefix.as_ref()),
            ("vercel-org-id", self.vercel_org_id.as_ref()),
            ("vercel-project-id", self.vercel_project_id.as_ref()),
        ];
        entries
            .into_iter()
            .filter_map(|(key, value)| value.map(|v| (key.to_string(), v.clone())))
            .collect()
    }
}

pub async fn execute(args: &PublishArgs) -> Result<()> {
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

    // Required parameters for firebase
    if targets.iter().any(|t| t == "firebase") && args.provider.firebase_app.is_none() {
        return Err(anyhow!(
            "Firebase app identifier is required for target 'firebase'"
        ));
    }

    let mut publish_arguments = parse_publish_args(&args.publish_args)?;
    for (key, value) in args.provider.to_arguments() {
        publish_arguments.entry(key).or_insert(value);
    }
    if let Some(version) = &args.app_version {
        publish_arguments
            .entry("app-version".to_string())
            .or_insert_with(|| version.clone());
    }

    // Like Dart, publishers see the environment plus the
    // `distribute_options.yaml` variables.
    let environment = global_variables(&DistributeOptions::load()?);
    for target in &targets {
        publish_artifact_with_env(
            &artifact_path,
            target,
            publish_arguments.clone(),
            environment.clone(),
        )?;
    }
    Ok(())
}

/// Resolves Dart-style `<target>-` prefixed publish arguments (e.g.
/// `github-repo`, `minio-bucket`) for `target`, mirroring
/// `UnifiedDistributor.publish`: a `<target>-<name>` key is exposed as
/// `<name>` and wins over an unprefixed `<name>`. Other keys are kept as-is so
/// unprefixed `--publish-arg` values keep working.
fn resolve_target_arguments(
    target: &str,
    publish_arguments: HashMap<String, String>,
) -> HashMap<String, String> {
    let prefix = format!("{}-", target);
    let mut resolved = publish_arguments.clone();
    for (key, value) in &publish_arguments {
        if let Some(stripped) = key.strip_prefix(&prefix) {
            resolved.insert(stripped.to_string(), value.clone());
        }
    }
    resolved
}

/// Renders upload progress while publishing, formatted like Dart's
/// `ProgressBar` (`Publishing to <target>: {bar} {value}/{total}
/// {percentage}%`, 40 cells). Written to stderr, only on a terminal.
fn progress_callback(target: &str) -> Option<PublishProgressCallback> {
    if !std::io::stderr().is_terminal() {
        return None;
    }
    let target = target.to_string();
    Some(Arc::new(move |sent: u64, total: u64| {
        if total == 0 {
            return;
        }
        const BAR_SIZE: usize = 40;
        let filled = ((sent as f64 / total as f64) * BAR_SIZE as f64)
            .round()
            .clamp(0.0, BAR_SIZE as f64) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(BAR_SIZE - filled));
        let percentage = sent as f64 * 100.0 / total as f64;
        let mut stderr = std::io::stderr();
        let _ = write!(
            stderr,
            "\rPublishing to {}: {} {}/{} {:.1}%",
            target, bar, sent, total, percentage
        );
        if sent >= total {
            let _ = writeln!(stderr);
        }
        let _ = stderr.flush();
    }))
}

/// Publishes with the process environment only (used by the local workflow
/// runner).
pub fn publish_artifact(
    artifact_path: &str,
    target: &str,
    publish_arguments: HashMap<String, String>,
) -> Result<String> {
    publish_artifact_with_env(
        artifact_path,
        target,
        publish_arguments,
        std::env::vars().collect(),
    )
}

/// Publishes `artifact_path` to `target` and prints
/// `Successfully published <url>` like Dart. Returns the publisher's result
/// message (the URL).
pub fn publish_artifact_with_env(
    artifact_path: &str,
    target: &str,
    publish_arguments: HashMap<String, String>,
    environment: HashMap<String, String>,
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
        environment,
    };

    let progress = progress_callback(&target);
    let publish = move || match target.as_str() {
        "s3" => S3Publisher::new().publish(publish_config, progress),
        "minio" => MinioPublisher::new().publish(publish_config, progress),
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
        "aur" => AurPublisher::new().publish(publish_config, progress),
        "custom" => CustomPublisher::new().publish(publish_config, progress),
        _ => Err(fastforge_app_publisher::PublishError::General(format!(
            "Unsupported publish target: `{}`. Currently supported: s3, minio, qiniu, oss, cos, fir, firebase, firebase-hosting, github, appstore, appgallery, playstore, pgyer, vercel, aur, custom",
            target
        ))),
    };
    // Publishers use blocking HTTP clients, which must not run (or be
    // dropped) on a tokio worker thread.
    let result = match tokio::runtime::Handle::try_current() {
        Ok(_) => tokio::task::block_in_place(publish),
        Err(_) => publish(),
    }
    .map_err(|e| anyhow!(e.to_string()))?;

    println!(
        "{}",
        bright_green(&format!("Successfully published {}", result.message))
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixed_arguments_win_for_their_target() {
        let args = HashMap::from([
            ("repo".to_string(), "a/b".to_string()),
            ("github-repo".to_string(), "c/d".to_string()),
            ("pgyer-password".to_string(), "x".to_string()),
        ]);
        let resolved = resolve_target_arguments("github", args);
        assert_eq!(resolved["repo"], "c/d");
        assert!(!resolved.contains_key("password"));
    }

    #[test]
    fn provider_flags_become_prefixed_arguments() {
        let provider = ProviderArgs {
            github_repo: Some("o/r".to_string()),
            github_release_draft: "false".to_string(),
            github_release_prerelease: "true".to_string(),
            ..Default::default()
        };
        let args = provider.to_arguments();
        assert_eq!(args["github-repo"], "o/r");
        assert_eq!(args["github-release-prerelease"], "true");
        assert!(!args.contains_key("pgyer-password"));
    }
}
