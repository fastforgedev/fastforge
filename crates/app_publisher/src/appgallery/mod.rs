use crate::common::{
    argument, argument_or_env, artifact_path, file_body, file_name, http_client, missing_env,
    to_publish_error,
};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct AppGalleryPublisher;

const PUBLISHER_NAME: &str = "appgallery";
const ENV_CLIENT_ID: &str = "APP_GALLERY_CLIENT_ID";
const ENV_CLIENT_SECRET: &str = "APP_GALLERY_CLIENT_SECRET";
const BASE_URL: &str = "https://connect-api.cloud.huawei.com";
const AGC_CONSOLE_URL: &str =
    "https://developer.huawei.com/consumer/cn/service/josp/agc/index.html";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct RetCode {
    code: i64,
    #[serde(default)]
    pub msg: Option<String>,
}

impl RetCode {
    fn check(self) -> Result<(), PublishError> {
        if self.code != 0 {
            return Err(PublishError::ApiError {
                status: self.code.to_string(),
                message: self.msg.unwrap_or_default(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct UploadHeader {
    name: String,
    value: String,
}

/// Dart reads `urlInfo.headers` as a JSON object; some responses carry an
/// array of `{name, value}` pairs instead. Both are accepted.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum UploadHeaders {
    Map(BTreeMap<String, Value>),
    List(Vec<UploadHeader>),
}

impl UploadHeaders {
    fn pairs(&self) -> Vec<(String, String)> {
        match self {
            Self::Map(map) => map
                .iter()
                .map(|(name, value)| {
                    let value = match value {
                        Value::String(value) => value.clone(),
                        other => other.to_string(),
                    };
                    (name.clone(), value)
                })
                .collect(),
            Self::List(list) => list
                .iter()
                .map(|header| (header.name.clone(), header.value.clone()))
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct UploadUrlInfo {
    url: String,
    #[serde(rename = "objectId")]
    object_id: String,
    headers: Option<UploadHeaders>,
}

#[derive(Debug, Deserialize)]
struct UploadUrlResponse {
    ret: RetCode,
    #[serde(rename = "urlInfo")]
    url_info: Option<UploadUrlInfo>,
}

#[derive(Debug, Deserialize)]
struct ApplyResponse {
    ret: RetCode,
}

impl AppPublisher for AppGalleryPublisher {
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
        let artifact_path = artifact_path(config)?;
        let file_name = file_name(artifact_path)?;

        let client_id = argument_or_env(config, &["client-id"], &[ENV_CLIENT_ID])
            .ok_or_else(|| missing_env(ENV_CLIENT_ID))?;
        let client_secret = argument_or_env(config, &["client-secret"], &[ENV_CLIENT_SECRET])
            .ok_or_else(|| missing_env(ENV_CLIENT_SECRET))?;
        let app_id = argument(config, &["app-id"])
            .ok_or_else(|| PublishError::General("Missing `app-id` arg".to_string()))?;

        let client = http_client()?;
        let token = get_access_token(&client, &client_id, &client_secret)?;
        let file_size = std::fs::metadata(artifact_path)
            .map_err(to_publish_error)?
            .len();
        let url_info = get_upload_url(&client, &client_id, &token, &app_id, &file_name, file_size)?;
        upload_file(&client, artifact_path, &url_info, on_progress)?;
        apply_upload(
            &client,
            &client_id,
            &token,
            &app_id,
            &file_name,
            &url_info.object_id,
        )?;

        Ok(PublishResult {
            success: true,
            message: AGC_CONSOLE_URL.to_string(),
        })
    }
}

fn get_access_token(
    client: &Client,
    client_id: &str,
    client_secret: &str,
) -> Result<String, PublishError> {
    let url = format!("{BASE_URL}/api/oauth2/v1/token");
    let body = json!({
        "grant_type": "client_credentials",
        "client_id": client_id,
        "client_secret": client_secret,
    });
    let response = client
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "AppGallery token request failed with status {}",
            response.status()
        )));
    }
    let resp: TokenResponse = response.json().map_err(to_publish_error)?;
    Ok(resp.access_token)
}

fn get_upload_url(
    client: &Client,
    client_id: &str,
    token: &str,
    app_id: &str,
    file_name: &str,
    content_length: u64,
) -> Result<UploadUrlInfo, PublishError> {
    let url = format!("{BASE_URL}/api/publish/v2/upload-url/for-obs");
    let response = client
        .get(&url)
        .header("client_id", client_id)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(CONTENT_TYPE, "application/json")
        .query(&[
            ("appId", app_id),
            ("fileName", file_name),
            ("contentLength", &content_length.to_string()),
        ])
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "AppGallery upload-url request failed with status {}",
            response.status()
        )));
    }
    let resp: UploadUrlResponse = response.json().map_err(to_publish_error)?;
    resp.ret.check()?;
    resp.url_info.ok_or_else(|| {
        PublishError::General("AppGallery upload-url response missing urlInfo.".to_string())
    })
}

fn upload_file(
    client: &Client,
    artifact_path: &str,
    url_info: &UploadUrlInfo,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<(), PublishError> {
    // The sized body sends `Content-Length: <file size>` like Dart does.
    let body = file_body(artifact_path, on_progress)?;
    let mut request = client.put(&url_info.url);
    for (name, value) in url_info
        .headers
        .as_ref()
        .map(UploadHeaders::pairs)
        .unwrap_or_default()
    {
        if !name.eq_ignore_ascii_case(CONTENT_LENGTH.as_str()) {
            request = request.header(name, value);
        }
    }
    let request = request.body(body);
    let response = request.send().map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "AppGallery file upload failed with status {}",
            response.status()
        )));
    }
    Ok(())
}

fn apply_upload(
    client: &Client,
    client_id: &str,
    token: &str,
    app_id: &str,
    file_name: &str,
    object_id: &str,
) -> Result<(), PublishError> {
    let url = format!("{BASE_URL}/api/publish/v3/app-package-info");
    let body = json!({
        "fileName": file_name,
        "objectId": object_id,
    });
    let response = client
        .put(&url)
        .header("client_id", client_id)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(CONTENT_TYPE, "application/json")
        .query(&[
            ("appId", app_id),
            ("releaseType", "1"),
            ("releasePhase", "0"),
        ])
        .json(&body)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "AppGallery apply-upload request failed with status {}",
            response.status()
        )));
    }
    let resp: ApplyResponse = response.json().map_err(to_publish_error)?;
    resp.ret.check()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_headers_accept_object_and_array() {
        let info: UploadUrlInfo = serde_json::from_str(
            r#"{"url":"https://obs","objectId":"o1","headers":{"Content-Type":"application/octet-stream","x-amz-date":"20250101"}}"#,
        )
        .unwrap();
        assert_eq!(
            info.headers.unwrap().pairs(),
            vec![
                (
                    "Content-Type".to_string(),
                    "application/octet-stream".to_string()
                ),
                ("x-amz-date".to_string(), "20250101".to_string()),
            ]
        );

        let info: UploadUrlInfo = serde_json::from_str(
            r#"{"url":"https://obs","objectId":"o1","headers":[{"name":"Host","value":"obs.example"}]}"#,
        )
        .unwrap();
        assert_eq!(
            info.headers.unwrap().pairs(),
            vec![("Host".to_string(), "obs.example".to_string())]
        );
    }

    #[test]
    fn ret_code_allows_missing_or_null_msg() {
        let upload: UploadUrlResponse = serde_json::from_str(
            r#"{"ret":{"code":0},"urlInfo":{"url":"https://obs","objectId":"o1"}}"#,
        )
        .unwrap();
        assert_eq!(upload.ret.msg, None);
        assert!(upload.ret.check().is_ok());

        let apply: ApplyResponse =
            serde_json::from_str(r#"{"ret":{"code":0,"msg":null}}"#).unwrap();
        assert_eq!(apply.ret.msg, None);
        assert!(apply.ret.check().is_ok());

        let missing_msg_err: ApplyResponse =
            serde_json::from_str(r#"{"ret":{"code":205524993}}"#).unwrap();
        match missing_msg_err.ret.check().unwrap_err() {
            PublishError::ApiError { status, message } => {
                assert_eq!(status, "205524993");
                assert_eq!(message, "");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }

        let with_msg_err: ApplyResponse =
            serde_json::from_str(r#"{"ret":{"code":205524993,"msg":"invalid token"}}"#).unwrap();
        match with_msg_err.ret.check().unwrap_err() {
            PublishError::ApiError { status, message } => {
                assert_eq!(status, "205524993");
                assert_eq!(message, "invalid token");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
