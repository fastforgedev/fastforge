use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::blocking::multipart::{Form, Part};
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::{Read, Result as IoResult};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub struct PgyerPublisher;

const PUBLISHER_NAME: &str = "pgyer";
const ENV_PGYER_API_KEY: &str = "PGYER_API_KEY";
const GET_COS_TOKEN_URL: &str = "https://www.pgyer.com/apiv2/app/getCOSToken";
const BUILD_INFO_URL: &str = "https://www.pgyer.com/apiv2/app/buildInfo";
const PGYER_APP_URL_PREFIX: &str = "http://www.pgyer.com";
const BUILD_INFO_PROCESSING_CODE: i64 = 1247;
const MAX_BUILD_INFO_RETRIES: usize = 10;
const BUILD_INFO_RETRY_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Deserialize)]
struct PgyerResponse<T> {
    code: i64,
    message: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct CosTokenData {
    endpoint: String,
    key: String,
    params: CosTokenParams,
}

#[derive(Debug, Deserialize)]
struct CosTokenParams {
    signature: String,
    #[serde(rename = "x-cos-security-token")]
    x_cos_security_token: String,
}

#[derive(Debug, Deserialize)]
struct BuildInfoData {
    #[serde(rename = "buildKey")]
    build_key: String,
}

impl AppPublisher for PgyerPublisher {
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
        let api_key = env::var(ENV_PGYER_API_KEY)
            .map_err(|_| PublishError::MissingEnv(ENV_PGYER_API_KEY.to_string()))?;
        let artifact_path = config
            .artifact_path
            .as_deref()
            .ok_or_else(|| PublishError::MissingArgument("artifact_path".to_string()))?;
        let build_type = file_extension(artifact_path).ok_or_else(|| {
            PublishError::General(format!(
                "Cannot infer build type from artifact path: {artifact_path}"
            ))
        })?;

        let client = Client::new();
        let token_data = self.get_cos_token(&client, &api_key, build_type, config)?;
        let upload_key = self.upload_app(&client, artifact_path, &token_data, on_progress)?;
        let build_info = self.get_build_info_with_retry(&client, &api_key, &upload_key)?;
        let build_key = build_info.build_key;

        Ok(PublishResult {
            success: true,
            message: format!("{PGYER_APP_URL_PREFIX}/{build_key}"),
        })
    }
}

impl PgyerPublisher {
    fn get_cos_token(
        &self,
        client: &Client,
        api_key: &str,
        build_type: &str,
        config: &PublishConfig,
    ) -> Result<CosTokenData, PublishError> {
        let mut form = Form::new()
            .text("_api_key", api_key.to_string())
            .text("buildType", build_type.to_string());

        // Optional pgyer API parameters, mirroring Dart's `PublishPgyerConfig`.
        // See https://www.pgyer.com/doc/view/api#fastUploadApp
        let optional_params = [
            ("oversea", "oversea"),
            ("install-type", "buildInstallType"),
            ("password", "buildPassword"),
            ("description", "buildDescription"),
            ("update-description", "buildUpdateDescription"),
            ("install-date", "buildInstallDate"),
            ("install-start-date", "buildInstallStartDate"),
            ("install-end-date", "buildInstallEndDate"),
            ("channel-shortcut", "buildChannelShortcut"),
        ];
        if let Some(arguments) = config.publish_arguments.as_ref() {
            for (arg_key, form_key) in optional_params {
                if let Some(value) = arguments
                    .get(arg_key)
                    .or_else(|| arguments.get(&format!("pgyer-{arg_key}")))
                    .filter(|v| !v.is_empty())
                {
                    form = form.text(form_key, value.clone());
                }
            }
        }
        let response = client
            .post(GET_COS_TOKEN_URL)
            .multipart(form)
            .send()
            .map_err(to_publish_error)?;
        let body: PgyerResponse<CosTokenData> = response.json().map_err(to_publish_error)?;

        if body.code != 0 {
            return Err(PublishError::ApiError {
                status: body.code.to_string(),
                message: body.message.unwrap_or_default(),
            });
        }
        body.data.ok_or_else(|| {
            PublishError::General("getCOSToken error: missing response data.".to_string())
        })
    }

    fn upload_app(
        &self,
        client: &Client,
        artifact_path: &str,
        token_data: &CosTokenData,
        on_progress: Option<&PublishProgressCallback>,
    ) -> Result<String, PublishError> {
        let file_name = file_name(artifact_path).ok_or_else(|| {
            PublishError::General(format!(
                "Cannot infer file name from artifact path: {artifact_path}"
            ))
        })?;
        let file = File::open(artifact_path).map_err(to_publish_error)?;
        let total_size = file.metadata().map_err(to_publish_error)?.len();
        let progress_reader = UploadProgressReader::new(file, total_size, on_progress.cloned());

        let form = Form::new()
            .text("key", token_data.key.clone())
            .text("signature", token_data.params.signature.clone())
            .text(
                "x-cos-security-token",
                token_data.params.x_cos_security_token.clone(),
            )
            .text("x-cos-meta-file-name", file_name.clone())
            .part(
                "file",
                Part::reader(progress_reader)
                    .file_name(file_name)
                    .mime_str("application/octet-stream")
                    .map_err(to_publish_error)?,
            );
        let response = client
            .post(&token_data.endpoint)
            .multipart(form)
            .send()
            .map_err(to_publish_error)?;

        if response.status() != StatusCode::NO_CONTENT {
            return Err(PublishError::HttpError(format!(
                "uploadApp error: unexpected status code {}",
                response.status()
            )));
        }

        Ok(token_data.key.clone())
    }

    fn get_build_info_with_retry(
        &self,
        client: &Client,
        api_key: &str,
        upload_key: &str,
    ) -> Result<BuildInfoData, PublishError> {
        for _ in 0..=MAX_BUILD_INFO_RETRIES {
            thread::sleep(BUILD_INFO_RETRY_INTERVAL);
            let response = client
                .get(BUILD_INFO_URL)
                .query(&[("_api_key", api_key), ("buildKey", upload_key)])
                .send()
                .map_err(to_publish_error)?;
            let body: PgyerResponse<BuildInfoData> = response.json().map_err(to_publish_error)?;

            if body.code == 0 {
                return body.data.ok_or_else(|| {
                    PublishError::General("getBuildInfo error: missing response data.".to_string())
                });
            }
            if body.code != BUILD_INFO_PROCESSING_CODE {
                return Err(PublishError::ApiError {
                    status: body.code.to_string(),
                    message: body.message.unwrap_or_default(),
                });
            }
        }

        Err(PublishError::General(
            "getBuildInfo error: Too many retries".to_string(),
        ))
    }
}

fn to_publish_error(error: impl std::fmt::Display) -> PublishError {
    PublishError::General(error.to_string())
}

fn file_extension(path: &str) -> Option<&str> {
    Path::new(path).extension().and_then(|ext| ext.to_str())
}

fn file_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

struct UploadProgressReader {
    file: File,
    sent: u64,
    total: u64,
    on_progress: Option<PublishProgressCallback>,
}

impl UploadProgressReader {
    fn new(file: File, total: u64, on_progress: Option<PublishProgressCallback>) -> Self {
        if let Some(callback) = &on_progress {
            callback(0, total);
        }
        Self {
            file,
            sent: 0,
            total,
            on_progress,
        }
    }
}

impl Read for UploadProgressReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let bytes_read = self.file.read(buf)?;
        if bytes_read > 0 {
            self.sent += bytes_read as u64;
            if let Some(callback) = &self.on_progress {
                callback(self.sent, self.total);
            }
        }
        Ok(bytes_read)
    }
}
