use crate::common::{argument, artifact_path, cli_command, run_streaming};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};

pub struct FirebasePublisher;

const PUBLISHER_NAME: &str = "firebase";
const ENV_FIREBASE_TOKEN: &str = "FIREBASE_TOKEN";
const FIREBASE_CONSOLE_URL: &str = "https://console.firebase.google.com/project/_/appdistribution";

impl AppPublisher for FirebasePublisher {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        PUBLISHER_NAME
    }

    fn is_supported_on_current_platform(&self) -> bool {
        true
    }

    fn perform_publish(
        &self,
        config: &PublishConfig,
        _on_progress: Option<&PublishProgressCallback>,
    ) -> Result<PublishResult, PublishError> {
        let artifact_path = artifact_path(config)?;
        let cmd_args = distribute_args(config, artifact_path)?;

        let output = run_streaming(cli_command("firebase").args(&cmd_args))
            .map_err(|e| PublishError::CommandFailed(format!("Failed to run firebase CLI: {e}")))?;

        if output.status.success() {
            Ok(PublishResult {
                success: true,
                message: FIREBASE_CONSOLE_URL.to_string(),
            })
        } else {
            Err(PublishError::General(format!(
                "{} - Upload of firebase failed",
                output.code()
            )))
        }
    }
}

/// Mirrors Dart's `PublishFirebaseConfig.parse` + `toFirebaseCliDistributeArgs`.
fn distribute_args(
    config: &PublishConfig,
    artifact_path: &str,
) -> Result<Vec<String>, PublishError> {
    let token = config.env_var(ENV_FIREBASE_TOKEN).ok_or_else(|| {
        PublishError::General(format!(
            "Missing `{ENV_FIREBASE_TOKEN}` environment variable. See:https://firebase.google.com/docs/cli?authuser=0#cli-ci-systems"
        ))
    })?;
    let app = argument(config, &["app"]).ok_or_else(|| {
        PublishError::General(
            "Missing app args. See:https://console.firebase.google.com/project/_/settings/general/?authuser=0"
                .to_string(),
        )
    })?;

    let mut cmd_args = vec![
        "appdistribution:distribute".to_string(),
        artifact_path.to_string(),
        "--app".to_string(),
        app,
        "--token".to_string(),
        token,
    ];
    for arg_name in [
        "release-notes",
        "release-notes-file",
        "testers",
        "testers-file",
        "groups",
        "groups-file",
    ] {
        if let Some(value) = argument(config, &[arg_name]) {
            cmd_args.push(format!("--{arg_name}"));
            cmd_args.push(value);
        }
    }
    Ok(cmd_args)
}
