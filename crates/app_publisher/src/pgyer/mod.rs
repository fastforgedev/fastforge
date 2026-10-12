use crate::common::{
    artifact_path, file_name, file_part, http_client, required_env, to_publish_error,
};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::blocking::multipart::Form;
use serde::Deserialize;
use serde_json::Value;
use std::thread;
use std::time::Duration;

pub struct PgyerPublisher;

const PUBLISHER_NAME: &str = "pgyer";
const ENV_PGYER_API_KEY: &str = "PGYER_API_KEY";
const GET_COS_TOKEN_URL: &str = "https://www.pgyer.com/apiv2/app/getCOSToken";
const BUILD_INFO_URL: &str = "https://www.pgyer.com/apiv2/app/buildInfo";
const PGYER_APP_URL_PREFIX: &str = "http://www.pgyer.com";
const BUILD_INFO_PROCESSING_CODE: i64 = 1247;
/// "应用发布失败": pgyer rejected the uploaded build.
const BUILD_INFO_FAILED_CODE: i64 = 1216;
/// "非法请求": also returned for a build key pgyer has not registered yet,
/// right after the upload.
const BUILD_INFO_INVALID_REQUEST_CODE: i64 = 1081;
const MAX_INVALID_REQUEST_RETRIES: usize = 5;
/// pgyer usually finishes processing within a minute; wait up to two.
const MAX_BUILD_INFO_RETRIES: usize = 40;
const BUILD_INFO_RETRY_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Deserialize)]
struct PgyerResponse<T> {
    code: i64,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParamValue {
    Int(i64),
    Text(String),
}

impl ParamValue {
    fn to_json(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Text(value) => Value::String(value.clone()).to_string(),
        }
    }
}

/// Mirrors Dart's `PublishPgyerConfig`.
/// See https://www.pgyer.com/doc/view/api#fastUploadApp
#[derive(Debug)]
struct PgyerConfig {
    api_key: String,
    /// `(form field, value)` in Dart's `toJson` order; `None` values removed.
    params: Vec<(&'static str, ParamValue)>,
}

impl PgyerConfig {
    fn parse(config: &PublishConfig) -> Result<Self, PublishError> {
        let api_key = required_env(config, ENV_PGYER_API_KEY)?;
        // (publish argument, form field, is integer)
        let fields: [(&str, &'static str, bool); 9] = [
            ("oversea", "oversea", true),
            ("install-type", "buildInstallType", true),
            ("password", "buildPassword", false),
            ("description", "buildDescription", false),
            ("update-description", "buildUpdateDescription", false),
            ("install-date", "buildInstallDate", true),
            ("install-start-date", "buildInstallStartDate", false),
            ("install-end-date", "buildInstallEndDate", false),
            ("channel-shortcut", "buildChannelShortcut", false),
        ];
        let params = fields
            .into_iter()
            .filter_map(|(arg_key, form_key, is_int)| {
                let raw = raw_argument(config, arg_key)?;
                let value = if is_int {
                    // Dart's `int.tryParse`: non-numeric values are dropped.
                    ParamValue::Int(raw.trim().parse::<i64>().ok()?)
                } else {
                    ParamValue::Text(raw)
                };
                Some((form_key, value))
            })
            .collect();
        Ok(Self { api_key, params })
    }

    /// Dart's `JsonEncoder.withIndent('  ').convert(config.toJson())`.
    fn to_pretty_json(&self) -> String {
        if self.params.is_empty() {
            return "{}".to_string();
        }
        let lines = self
            .params
            .iter()
            .map(|(key, value)| format!("  \"{key}\": {}", value.to_json()))
            .collect::<Vec<_>>()
            .join(",\n");
        format!("{{\n{lines}\n}}")
    }
}

/// Publish argument as given (empty strings kept, like Dart's `_parseString`).
fn raw_argument(config: &PublishConfig, key: &str) -> Option<String> {
    let arguments = config.publish_arguments.as_ref()?;
    arguments
        .get(key)
        .or_else(|| arguments.get(&format!("pgyer-{key}")))
        .cloned()
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
        let artifact_path = artifact_path(config)?;
        let pgyer_config = PgyerConfig::parse(config)?;
        println!("config:\n{}", pgyer_config.to_pretty_json());

        // Dart: `filePath.split('.').last`
        let build_type = artifact_path.rsplit('.').next().unwrap_or_default();

        let client = http_client()?;
        let token_data = get_cos_token(&client, &pgyer_config, build_type)?;
        let upload_key = upload_app(&client, artifact_path, &token_data, on_progress)?;
        let build_info = get_build_info_with_retry(
            &client,
            BUILD_INFO_URL,
            BUILD_INFO_RETRY_INTERVAL,
            &pgyer_config.api_key,
            &upload_key,
        )?;
        let build_key = build_info.build_key;

        Ok(PublishResult {
            success: true,
            message: format!("{PGYER_APP_URL_PREFIX}/{build_key}"),
        })
    }
}

fn get_cos_token(
    client: &Client,
    config: &PgyerConfig,
    build_type: &str,
) -> Result<CosTokenData, PublishError> {
    let mut form = Form::new()
        .text("_api_key", config.api_key.clone())
        .text("buildType", build_type.to_string());
    // Dart's `_addOptionalParameter`: empty strings are not sent.
    for (key, value) in &config.params {
        match value {
            ParamValue::Int(value) => form = form.text(*key, value.to_string()),
            ParamValue::Text(value) if !value.is_empty() => form = form.text(*key, value.clone()),
            ParamValue::Text(_) => {}
        }
    }
    let response = client
        .post(GET_COS_TOKEN_URL)
        .multipart(form)
        .send()
        .map_err(to_publish_error)?;
    let text = response.text().map_err(to_publish_error)?;
    let body: PgyerResponse<CosTokenData> = serde_json::from_str(&text)
        .map_err(|error| PublishError::General(format!("getCOSToken error: {error}: {text}")))?;
    if body.code != 0 {
        return Err(PublishError::General(format!("getCOSToken error: {text}")));
    }
    body.data
        .ok_or_else(|| PublishError::General(format!("getCOSToken error: {text}")))
}

fn upload_app(
    client: &Client,
    artifact_path: &str,
    token_data: &CosTokenData,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<String, PublishError> {
    let file_name = file_name(artifact_path)?;
    let form = Form::new()
        .text("key", token_data.key.clone())
        .text("signature", token_data.params.signature.clone())
        .text(
            "x-cos-security-token",
            token_data.params.x_cos_security_token.clone(),
        )
        .text("x-cos-meta-file-name", file_name.clone())
        .part("file", file_part(artifact_path, &file_name, on_progress)?);
    let response = client
        .post(&token_data.endpoint)
        .multipart(form)
        .send()
        .map_err(to_publish_error)?;

    if response.status() != StatusCode::NO_CONTENT {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        return Err(PublishError::General(format!(
            "UploadApp error: unexpected status code {status} {text}"
        )));
    }

    Ok(token_data.key.clone())
}

fn get_build_info_with_retry(
    client: &Client,
    url: &str,
    interval: Duration,
    api_key: &str,
    upload_key: &str,
) -> Result<BuildInfoData, PublishError> {
    let mut try_count = 0;
    loop {
        if try_count > MAX_BUILD_INFO_RETRIES {
            return Err(PublishError::General(
                "getBuildInfo error :Too many retries".to_string(),
            ));
        }
        thread::sleep(interval);
        let response = client
            .get(url)
            .query(&[("_api_key", api_key), ("buildKey", upload_key)])
            .send()
            .map_err(to_publish_error)?;
        let text = response.text().map_err(to_publish_error)?;
        let body: PgyerResponse<BuildInfoData> = serde_json::from_str(&text).map_err(|error| {
            PublishError::General(format!("getBuildInfo error: {error}: {text}"))
        })?;

        match body.code {
            0 => {
                return body
                    .data
                    .ok_or_else(|| PublishError::General(format!("getBuildInfo error: {text}")));
            }
            BUILD_INFO_PROCESSING_CODE => {
                try_count += 1;
                println!("应用发布信息获取中，请稍等 {try_count}");
            }
            BUILD_INFO_INVALID_REQUEST_CODE if try_count < MAX_INVALID_REQUEST_RETRIES => {
                try_count += 1;
                println!("应用发布信息获取中，请稍等 {try_count}");
            }
            BUILD_INFO_FAILED_CODE => {
                return Err(PublishError::General(format!(
                    "getBuildInfo error: pgyer failed to publish the uploaded build; check that the file is a valid, signed package: {text}"
                )));
            }
            _ => return Err(PublishError::General(format!("getBuildInfo error: {text}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serves one canned JSON body per request on a local port.
    fn serve(bodies: Vec<&'static str>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/buildInfo", listener.local_addr().unwrap());
        thread::spawn(move || {
            for body in bodies {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        url
    }

    #[test]
    fn build_info_waits_through_unregistered_and_processing_builds() {
        let url = serve(vec![
            r#"{"code":1081,"message":"error request"}"#,
            r#"{"code":1247,"message":"processing"}"#,
            r#"{"code":0,"data":{"buildKey":"abc123"}}"#,
        ]);
        let client = Client::new();
        let info =
            get_build_info_with_retry(&client, &url, Duration::ZERO, "key", "upload").unwrap();
        assert_eq!(info.build_key, "abc123");
    }

    #[test]
    fn build_info_reports_a_rejected_build() {
        let url = serve(vec![r#"{"code":1216,"message":"failed"}"#]);
        let error = get_build_info_with_retry(&Client::new(), &url, Duration::ZERO, "k", "u")
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to publish"), "{error}");
    }

    fn config_with(arguments: &[(&str, &str)]) -> PublishConfig {
        PublishConfig {
            publish_arguments: Some(
                arguments
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ),
            environment: [(ENV_PGYER_API_KEY.to_string(), "key".to_string())].into(),
            ..Default::default()
        }
    }

    #[test]
    fn integer_params_are_parsed_and_invalid_ones_dropped() {
        let config = PgyerConfig::parse(&config_with(&[
            ("oversea", "1"),
            ("install-type", "abc"),
            ("install-date", "2"),
            ("description", ""),
        ]))
        .unwrap();
        assert_eq!(
            config.params,
            vec![
                ("oversea", ParamValue::Int(1)),
                ("buildDescription", ParamValue::Text(String::new())),
                ("buildInstallDate", ParamValue::Int(2)),
            ]
        );
        assert_eq!(
            config.to_pretty_json(),
            "{\n  \"oversea\": 1,\n  \"buildDescription\": \"\",\n  \"buildInstallDate\": 2\n}"
        );
    }

    #[test]
    fn missing_api_key_uses_dart_message() {
        if std::env::var(ENV_PGYER_API_KEY).is_ok_and(|v| !v.is_empty()) {
            return;
        }
        let error = PgyerConfig::parse(&PublishConfig::default()).unwrap_err();
        {
            assert_eq!(
                error.to_string(),
                "Missing `PGYER_API_KEY` environment variable."
            );
        }
    }
}
