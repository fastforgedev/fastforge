use crate::common::{
    argument_or_env, artifact_path, cli_command, extract_url_after, run_streaming,
};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use serde_json::json;
use std::fs;
use std::path::Path;

pub struct VercelPublisher;

const PUBLISHER_NAME: &str = "vercel";
const ENV_VERCEL_ORG_ID: &str = "VERCEL_ORG_ID";
const ENV_VERCEL_PROJECT_ID: &str = "VERCEL_PROJECT_ID";

impl AppPublisher for VercelPublisher {
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
        let org_id = argument_or_env(config, &["org-id", "vercel-org-id"], &[ENV_VERCEL_ORG_ID])
            .ok_or_else(|| PublishError::General("Missing `org-id` config.".to_string()))?;
        let project_id = argument_or_env(
            config,
            &["project-id", "vercel-project-id"],
            &[ENV_VERCEL_PROJECT_ID],
        )
        .ok_or_else(|| PublishError::General("Missing `project-id` config.".to_string()))?;

        let vercel_dir = Path::new(artifact_path).join(".vercel");
        fs::create_dir_all(&vercel_dir).map_err(|e| {
            PublishError::General(format!("Failed to create .vercel directory: {e}"))
        })?;
        let project_json = json!({ "orgId": org_id, "projectId": project_id });
        fs::write(vercel_dir.join("project.json"), project_json.to_string()).map_err(|e| {
            PublishError::General(format!("Failed to write .vercel/project.json: {e}"))
        })?;

        let output = run_streaming(
            cli_command("vercel")
                .arg("--prod")
                .current_dir(artifact_path),
        )
        .map_err(|e| PublishError::CommandFailed(format!("Failed to run vercel CLI: {e}")))?;
        if !output.status.success() {
            return Err(PublishError::CommandFailed(format!(
                "vercel deploy failed ({})",
                output.code()
            )));
        }

        // Dart: `(?<=Production: )\bhttps?:\/\/\S+\b` on stderr, '' when
        // absent. stdout is checked as a fallback.
        let url = extract_url_after(&output.stderr, "Production: ")
            .or_else(|| extract_url_after(&output.stdout, "Production: "))
            .unwrap_or_default();
        Ok(PublishResult {
            success: true,
            message: url,
        })
    }
}
