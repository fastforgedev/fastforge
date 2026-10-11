use crate::common::{
    argument_or_env, artifact_path, file_body, http_client, missing_env, resolve_app_version,
    to_publish_error,
};
use chrono::Utc;
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::File;
use std::path::Path;

pub struct PlayStorePublisher;

const PUBLISHER_NAME: &str = "playstore";
const PLAY_CONSOLE_URL_PREFIX: &str = "https://play.google.com/console/u/0/developers";
const ENV_PLAYSTORE_CREDENTIALS_FILE: &str = "PLAYSTORE_CREDENTIALS";
const ANDROID_PUBLISHER_SCOPE: &str = "https://www.googleapis.com/auth/androidpublisher";
const DEFAULT_GOOGLE_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";

impl AppPublisher for PlayStorePublisher {
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
        on_progress: Option<&PublishProgressCallback>,
    ) -> Result<PublishResult, PublishError> {
        let app_version = resolve_app_version(config)?.map(|version| version.text);
        let artifact_path = artifact_path(config)?;
        ensure_bundle_extension(artifact_path)?;
        let publish_config = PlayStoreConfig::from_config(config)?;
        let credentials = ServiceAccountCredentials::from_file(&publish_config.credentials_file)?;
        let client = http_client()?;
        let access_token = fetch_access_token(&client, &credentials)?;

        let edit_id = insert_edit(&client, &access_token, &publish_config.package_name)?;
        let version_code = upload_bundle(
            &client,
            &access_token,
            &publish_config.package_name,
            &edit_id,
            artifact_path,
            on_progress,
        )?;

        if let Some(track) = publish_config.track.as_deref().filter(|t| !t.is_empty()) {
            let release_name = build_release_name(artifact_path, app_version.as_deref());
            update_track(
                &client,
                &access_token,
                &publish_config.package_name,
                &edit_id,
                track,
                &release_name,
                version_code,
            )?;
        }

        commit_edit(
            &client,
            &access_token,
            &publish_config.package_name,
            &edit_id,
        )?;
        Ok(PublishResult {
            success: true,
            message: format!(
                "{PLAY_CONSOLE_URL_PREFIX}/?app={}",
                publish_config.package_name
            ),
        })
    }
}

#[derive(Debug)]
struct PlayStoreConfig {
    credentials_file: String,
    package_name: String,
    track: Option<String>,
}

impl PlayStoreConfig {
    fn from_config(config: &PublishConfig) -> Result<Self, PublishError> {
        let credentials_file = argument_or_env(
            config,
            &["credentials-file", "playstore-credentials-file"],
            &[ENV_PLAYSTORE_CREDENTIALS_FILE],
        )
        .ok_or_else(|| missing_env(ENV_PLAYSTORE_CREDENTIALS_FILE))?;
        let package_name = required_value(
            config,
            &["package-name", "playstore-package-name"],
            &[],
            "PlayStore package name",
        )?;
        let track = optional_value(config, &["track", "playstore-track"], &[]);

        Ok(Self {
            credentials_file,
            package_name,
            track,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ServiceAccountCredentials {
    client_email: String,
    private_key: String,
    token_uri: Option<String>,
}

impl ServiceAccountCredentials {
    fn from_file(path: &str) -> Result<Self, PublishError> {
        let file = File::open(path).map_err(to_publish_error)?;
        serde_json::from_reader(file).map_err(to_publish_error)
    }
}

#[derive(Debug, Serialize)]
struct AccessTokenClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct InsertEditResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct UploadBundleResponse {
    #[serde(rename = "versionCode")]
    version_code: i64,
}

fn fetch_access_token(
    client: &Client,
    credentials: &ServiceAccountCredentials,
) -> Result<String, PublishError> {
    let token_uri = credentials
        .token_uri
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_GOOGLE_TOKEN_URI);
    let issued_at = Utc::now().timestamp();
    let claims = AccessTokenClaims {
        iss: &credentials.client_email,
        scope: ANDROID_PUBLISHER_SCOPE,
        aud: token_uri,
        iat: issued_at,
        exp: issued_at + 3600,
    };
    let jwt = encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(credentials.private_key.as_bytes()).map_err(to_publish_error)?,
    )
    .map_err(to_publish_error)?;

    let response = client
        .post(token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ])
        .send()
        .map_err(to_publish_error)?;

    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "Failed to get PlayStore access token: status={}",
            response.status()
        )));
    }

    let body: AccessTokenResponse = response.json().map_err(to_publish_error)?;
    Ok(body.access_token)
}

fn insert_edit(
    client: &Client,
    access_token: &str,
    package_name: &str,
) -> Result<String, PublishError> {
    let url = format!(
        "https://androidpublisher.googleapis.com/androidpublisher/v3/applications/{package_name}/edits"
    );
    let response = client
        .post(url)
        .bearer_auth(access_token)
        .json(&json!({}))
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "Failed to create PlayStore edit: status={}",
            response.status()
        )));
    }
    let body: InsertEditResponse = response.json().map_err(to_publish_error)?;
    Ok(body.id)
}

fn upload_bundle(
    client: &Client,
    access_token: &str,
    package_name: &str,
    edit_id: &str,
    artifact_path: &str,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<i64, PublishError> {
    let url = format!(
        "https://androidpublisher.googleapis.com/upload/androidpublisher/v3/applications/{package_name}/edits/{edit_id}/bundles?uploadType=media"
    );
    let body = file_body(artifact_path, on_progress)?;
    let response = client
        .post(url)
        .bearer_auth(access_token)
        .header("content-type", "application/octet-stream")
        .body(body)
        .send()
        .map_err(to_publish_error)?;

    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "Failed to upload bundle to PlayStore: status={}",
            response.status()
        )));
    }

    let body: UploadBundleResponse = response.json().map_err(to_publish_error)?;
    Ok(body.version_code)
}

fn update_track(
    client: &Client,
    access_token: &str,
    package_name: &str,
    edit_id: &str,
    track: &str,
    release_name: &str,
    version_code: i64,
) -> Result<(), PublishError> {
    let url = format!(
        "https://androidpublisher.googleapis.com/androidpublisher/v3/applications/{package_name}/edits/{edit_id}/tracks/{track}"
    );
    let body = json!({
        "track": track,
        "releases": [{
            "name": release_name,
            "versionCodes": [version_code.to_string()],
            "status": "completed"
        }]
    });
    let response = client
        .put(url)
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "Failed to update PlayStore track `{track}`: status={}",
            response.status()
        )));
    }
    Ok(())
}

fn commit_edit(
    client: &Client,
    access_token: &str,
    package_name: &str,
    edit_id: &str,
) -> Result<(), PublishError> {
    let url = format!(
        "https://androidpublisher.googleapis.com/androidpublisher/v3/applications/{package_name}/edits/{edit_id}:commit"
    );
    let response = client
        .post(url)
        .bearer_auth(access_token)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "Failed to commit PlayStore edit: status={}",
            response.status()
        )));
    }
    Ok(())
}

fn build_release_name(path: &str, app_version: Option<&str>) -> String {
    if let Some(file_name) = Path::new(path).file_name().and_then(|name| name.to_str()) {
        let parts: Vec<&str> = file_name.split('-').collect();
        if let Some(version_part) = parts.get(1) {
            let versions: Vec<&str> = version_part.split('+').collect();
            if versions.len() == 2 {
                return format!("{} ({})", versions[1], versions[0]);
            }
        }
    }
    app_version.unwrap_or("release").to_string()
}

fn ensure_bundle_extension(path: &str) -> Result<(), PublishError> {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if extension != "aab" {
        return Err(PublishError::General(format!(
            "PlayStore publisher only supports `.aab` bundles, got `{path}`."
        )));
    }
    Ok(())
}

fn required_value(
    config: &PublishConfig,
    argument_keys: &[&str],
    env_keys: &[&str],
    field_name: &str,
) -> Result<String, PublishError> {
    optional_value(config, argument_keys, env_keys)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| PublishError::MissingArgument(field_name.to_string()))
}

fn optional_value(
    config: &PublishConfig,
    argument_keys: &[&str],
    env_keys: &[&str],
) -> Option<String> {
    argument_or_env(config, argument_keys, env_keys)
}
