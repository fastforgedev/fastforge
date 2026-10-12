use crate::common::{argument_or_env, artifact_path, extract_url_after, run_streaming};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct FirebaseHostingPublisher;

const PUBLISHER_NAME: &str = "firebase-hosting";
const ENV_FIREBASE_TOKEN: &str = "FIREBASE_TOKEN";
const ENV_FIREBASE_PROJECT_ID: &str = "FIREBASE_PROJECT_ID";

impl AppPublisher for FirebaseHostingPublisher {
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
        let project_id = argument_or_env(
            config,
            &["project-id", "firebase-project-id"],
            &[ENV_FIREBASE_PROJECT_ID],
        )
        .ok_or_else(|| PublishError::General("Missing `project-id` config.".to_string()))?;

        let directory = Path::new(artifact_path);
        fs::create_dir_all(directory)?;
        let firebaserc = json!({ "projects": { "default": project_id } });
        fs::write(directory.join(".firebaserc"), firebaserc.to_string())
            .map_err(|e| PublishError::General(format!("Failed to write .firebaserc: {e}")))?;
        let firebase_json = json!({ "hosting": hosting_config(Path::new("firebase.json")) });
        fs::write(directory.join("firebase.json"), firebase_json.to_string())
            .map_err(|e| PublishError::General(format!("Failed to write firebase.json: {e}")))?;

        let mut cmd = Command::new("firebase");
        cmd.arg("deploy").current_dir(artifact_path);
        // Rust extra: pass `FIREBASE_TOKEN` when available.
        if let Some(token) = config.env_var(ENV_FIREBASE_TOKEN) {
            cmd.arg("--token").arg(token);
        }

        let output = run_streaming(&mut cmd)
            .map_err(|e| PublishError::CommandFailed(format!("Failed to run firebase CLI: {e}")))?;
        if !output.status.success() {
            return Err(PublishError::CommandFailed(format!(
                "firebase deploy failed ({})",
                output.code()
            )));
        }

        // Dart: `(?<=Hosting URL: )\bhttps?:\/\/\S+\b` on stdout, '' when absent.
        let url = extract_url_after(&output.stdout, "Hosting URL: ").unwrap_or_default();
        Ok(PublishResult {
            success: true,
            message: url,
        })
    }
}

/// The `hosting` configuration to deploy the artifact with: the project's
/// own from `firebase.json` when it has one, so headers, redirects,
/// rewrites and cleanUrls apply, served from the artifact directory. Deploy
/// hooks and a `target` (which needs the project's `.firebaserc`) are left
/// out, since the deploy runs in the artifact directory. Of several sites,
/// the first one is used.
fn hosting_config(project_firebase_json: &Path) -> Value {
    let project = fs::read_to_string(project_firebase_json)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok());
    let hosting = match project.as_ref().map(|config| &config["hosting"]) {
        Some(Value::Object(hosting)) => Some(hosting.clone()),
        Some(Value::Array(sites)) => sites.iter().find_map(|site| site.as_object().cloned()),
        _ => None,
    };
    let mut hosting = hosting.unwrap_or_else(Map::new);
    for key in [
        "target",
        "predeploy",
        "postdeploy",
        "source",
        "frameworksBackend",
    ] {
        hosting.remove(key);
    }
    hosting.insert("public".to_string(), json!("."));
    let mut ignore = match hosting.remove("ignore") {
        Some(Value::Array(ignore)) => ignore,
        _ => Vec::new(),
    };
    for file in ["firebase.json", ".firebaserc"] {
        if !ignore.iter().any(|entry| entry == file) {
            ignore.push(json!(file));
        }
    }
    hosting.insert("ignore".to_string(), Value::Array(ignore));
    Value::Object(hosting)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosting_defaults_to_serving_the_artifact() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            hosting_config(&dir.path().join("firebase.json")),
            json!({ "public": ".", "ignore": ["firebase.json", ".firebaserc"] })
        );
    }

    #[test]
    fn hosting_keeps_the_project_headers_and_rewrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("firebase.json");
        fs::write(
            &path,
            json!({
                "hosting": [{
                    "target": "app",
                    "public": "build/web",
                    "ignore": ["**/.*"],
                    "predeploy": ["flutter build web"],
                    "headers": [{ "source": "**/*.js", "headers": [{ "key": "Cache-Control", "value": "max-age=31536000" }] }],
                    "rewrites": [{ "source": "**", "destination": "/index.html" }]
                }]
            })
            .to_string(),
        )
        .unwrap();
        let hosting = hosting_config(&path);
        assert_eq!(hosting["public"], ".");
        assert_eq!(
            hosting["ignore"],
            json!(["**/.*", "firebase.json", ".firebaserc"])
        );
        assert_eq!(
            hosting["headers"][0]["headers"][0]["value"],
            "max-age=31536000"
        );
        assert_eq!(hosting["rewrites"][0]["destination"], "/index.html");
        assert!(hosting.get("target").is_none() && hosting.get("predeploy").is_none());
    }
}
