use chrono::Utc;
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use hmac::{Hmac, Mac};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{Read, Result as IoResult};
use std::path::Path;

mod cos;
mod oss;
mod qiniu;

pub use cos::CosPublisher;
pub use oss::OssPublisher;
pub use qiniu::QiniuPublisher;

pub struct S3Publisher;

const PUBLISHER_NAME: &str = "s3";
const S3_SERVICE: &str = "s3";
const S3_REQUEST: &str = "aws4_request";
const ENV_S3_ENDPOINT: &str = "S3_ENDPOINT";
const ENV_S3_REGION: &str = "S3_REGION";
const ENV_S3_ACCESS_KEY: &str = "S3_ACCESS_KEY";
const ENV_S3_SECRET_KEY: &str = "S3_SECRET_KEY";
const ENV_S3_BUCKET: &str = "S3_BUCKET";
const ENV_S3_KEY_PREFIX: &str = "S3_KEY_PREFIX";
const ENV_S3_PUBLIC_BASE_URL: &str = "S3_PUBLIC_BASE_URL";
const ENV_S3_FORCE_PATH_STYLE: &str = "S3_FORCE_PATH_STYLE";
const ENV_AWS_REGION: &str = "AWS_REGION";
const ENV_AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const ENV_AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const ENV_AWS_SESSION_TOKEN: &str = "AWS_SESSION_TOKEN";
// MinIO aliases (mirror Dart's `AppPackagePublisherMinio` configuration)
const ENV_MINIO_ENDPOINT: &str = "MINIO_ENDPOINT";
const ENV_MINIO_ACCESS_KEY: &str = "MINIO_ACCESS_KEY";
const ENV_MINIO_SECRET_KEY: &str = "MINIO_SECRET_KEY";
const DEFAULT_REGION: &str = "us-east-1";

type HmacSha256 = Hmac<Sha256>;
const AWS_URI_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

impl AppPublisher for S3Publisher {
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
        let artifact_path = config
            .artifact_path
            .as_deref()
            .ok_or_else(|| PublishError::MissingArgument("artifact_path".to_string()))?;
        let options = S3PublishOptions::from_config(config)?;
        upload_artifact(&options, artifact_path, on_progress)
    }
}

fn upload_artifact(
    options: &S3PublishOptions,
    artifact_path: &str,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<PublishResult, PublishError> {
    let artifact_name = file_name(artifact_path).ok_or_else(|| {
        PublishError::General(format!(
            "Cannot infer file name from artifact path: {artifact_path}"
        ))
    })?;
    let key = compose_object_key(options.key_prefix.as_deref(), &artifact_name);
    let payload_hash = sha256_file_hex(artifact_path)?;
    let datetime = Utc::now();
    let amz_date = datetime.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = datetime.format("%Y%m%d").to_string();
    let object_path = build_object_path(options.force_path_style, &options.bucket, &key);
    let endpoint = normalize_endpoint(&options.endpoint);
    let url = build_upload_url(&endpoint, options.force_path_style, &options.bucket, &key)?;
    let host = build_host(&endpoint, options.force_path_style, &options.bucket)?;
    let credential_scope = format!(
        "{}/{}/{}/{}",
        date_stamp, options.region, S3_SERVICE, S3_REQUEST
    );
    let canonical_headers = if options.session_token.is_some() {
        format!(
            "host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\nx-amz-security-token:{}\n",
            options.session_token.as_deref().unwrap_or_default()
        )
    } else {
        format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n")
    };
    let signed_headers = if options.session_token.is_some() {
        "host;x-amz-content-sha256;x-amz-date;x-amz-security-token"
    } else {
        "host;x-amz-content-sha256;x-amz-date"
    };
    let canonical_request =
        format!("PUT\n{object_path}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let canonical_request_hash = sha256_hex(canonical_request.as_bytes());
    let string_to_sign =
        format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{canonical_request_hash}");
    let signing_key = derive_signing_key(&options.secret_key, &date_stamp, &options.region)?;
    let signature = hmac_hex(&signing_key, &string_to_sign)?;
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        options.access_key, credential_scope, signed_headers, signature
    );

    let file = File::open(artifact_path).map_err(to_publish_error)?;
    let total_size = file.metadata().map_err(to_publish_error)?.len();
    let body_reader = UploadProgressReader::new(file, total_size, on_progress.cloned());

    let client = Client::new();
    let mut request = client
        .put(&url)
        .header("host", host)
        .header("x-amz-content-sha256", payload_hash)
        .header("x-amz-date", amz_date)
        .header("authorization", authorization)
        .body(reqwest::blocking::Body::sized(body_reader, total_size));

    if let Some(session_token) = &options.session_token {
        request = request.header("x-amz-security-token", session_token);
    }

    let response = request.send().map_err(to_publish_error)?;
    if response.status() != StatusCode::OK && response.status() != StatusCode::NO_CONTENT {
        return Err(PublishError::HttpError(format!(
            "S3 upload failed with status {}",
            response.status()
        )));
    }

    let url = build_public_url(options, &endpoint, &key)?;
    Ok(PublishResult {
        success: true,
        message: url,
    })
}

struct S3PublishOptions {
    endpoint: String,
    region: String,
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
    bucket: String,
    key_prefix: Option<String>,
    public_base_url: Option<String>,
    force_path_style: bool,
}

impl S3PublishOptions {
    fn from_config(config: &PublishConfig) -> Result<Self, PublishError> {
        let endpoint = required_value(
            config,
            &["endpoint", "s3-endpoint", "minio-endpoint"],
            &[ENV_S3_ENDPOINT, ENV_MINIO_ENDPOINT],
            "S3 endpoint",
        )?;
        let region = optional_value(
            config,
            &["region", "s3-region", "minio-region"],
            &[ENV_S3_REGION, ENV_AWS_REGION],
        )
        .unwrap_or_else(|| DEFAULT_REGION.to_string());
        let access_key = required_value(
            config,
            &["access-key", "s3-access-key", "minio-access-key"],
            &[ENV_S3_ACCESS_KEY, ENV_AWS_ACCESS_KEY_ID, ENV_MINIO_ACCESS_KEY],
            "S3 access key",
        )?;
        let secret_key = required_value(
            config,
            &["secret-key", "s3-secret-key", "minio-secret-key"],
            &[ENV_S3_SECRET_KEY, ENV_AWS_SECRET_ACCESS_KEY, ENV_MINIO_SECRET_KEY],
            "S3 secret key",
        )?;
        let bucket = required_value(
            config,
            &["bucket", "s3-bucket", "minio-bucket"],
            &[ENV_S3_BUCKET],
            "S3 bucket",
        )?;
        let key_prefix = optional_value(
            config,
            &["savekey-prefix", "key-prefix", "s3-key-prefix", "minio-savekey-prefix"],
            &[ENV_S3_KEY_PREFIX],
        );
        let public_base_url = optional_value(
            config,
            &["public-base-url", "bucket-domain", "s3-public-base-url"],
            &[ENV_S3_PUBLIC_BASE_URL],
        );
        let force_path_style = optional_value(
            config,
            &["force-path-style", "s3-force-path-style"],
            &[ENV_S3_FORCE_PATH_STYLE],
        )
        .map(|v| parse_bool(&v))
        .transpose()?
        .unwrap_or(true);
        let session_token = optional_value(
            config,
            &["session-token", "s3-session-token"],
            &[ENV_AWS_SESSION_TOKEN],
        );

        Ok(Self {
            endpoint,
            region,
            access_key,
            secret_key,
            session_token,
            bucket,
            key_prefix,
            public_base_url,
            force_path_style,
        })
    }
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
    config
        .publish_arguments
        .as_ref()
        .and_then(|arguments| {
            argument_keys
                .iter()
                .find_map(|key| arguments.get(*key).cloned())
        })
        .or_else(|| env_keys.iter().find_map(|key| env::var(key).ok()))
}

fn normalize_endpoint(endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        endpoint.to_string()
    } else {
        format!("https://{endpoint}")
    }
}

fn build_host(
    endpoint: &str,
    force_path_style: bool,
    bucket: &str,
) -> Result<String, PublishError> {
    let parsed = reqwest::Url::parse(endpoint).map_err(to_publish_error)?;
    let endpoint_host = parsed
        .host_str()
        .ok_or_else(|| PublishError::General("Invalid endpoint: missing host.".to_string()))?;
    let host = if force_path_style {
        endpoint_host.to_string()
    } else {
        format!("{bucket}.{endpoint_host}")
    };
    if let Some(port) = parsed.port() {
        Ok(format!("{host}:{port}"))
    } else {
        Ok(host)
    }
}

fn build_upload_url(
    endpoint: &str,
    force_path_style: bool,
    bucket: &str,
    key: &str,
) -> Result<String, PublishError> {
    let parsed = reqwest::Url::parse(endpoint).map_err(to_publish_error)?;
    let scheme = parsed.scheme();
    let host = parsed
        .host_str()
        .ok_or_else(|| PublishError::General("Invalid endpoint: missing host.".to_string()))?;
    let port_suffix = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    let encoded_key = encode_object_key(key);
    let url = if force_path_style {
        format!("{scheme}://{host}{port_suffix}/{bucket}/{encoded_key}")
    } else {
        format!("{scheme}://{bucket}.{host}{port_suffix}/{encoded_key}")
    };
    Ok(url)
}

fn build_object_path(force_path_style: bool, bucket: &str, key: &str) -> String {
    let encoded_key = encode_object_key(key);
    if force_path_style {
        format!("/{bucket}/{encoded_key}")
    } else {
        format!("/{encoded_key}")
    }
}

fn build_public_url(
    options: &S3PublishOptions,
    endpoint: &str,
    key: &str,
) -> Result<String, PublishError> {
    if let Some(base_url) = &options.public_base_url {
        let base = base_url.trim_end_matches('/');
        return Ok(format!("{base}/{}", encode_object_key(key)));
    }
    build_upload_url(endpoint, options.force_path_style, &options.bucket, key)
}

fn compose_object_key(prefix: Option<&str>, file_name: &str) -> String {
    match prefix {
        Some(prefix) if !prefix.trim().is_empty() => {
            let normalized = prefix.trim_matches('/');
            format!("{normalized}/{file_name}")
        }
        _ => file_name.to_string(),
    }
}

fn encode_object_key(key: &str) -> String {
    key.split('/')
        .map(|segment| utf8_percent_encode(segment, AWS_URI_ENCODE_SET).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn sha256_file_hex(path: &str) -> Result<String, PublishError> {
    let mut file = File::open(path).map_err(to_publish_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer).map_err(to_publish_error)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

fn derive_signing_key(secret_key: &str, date: &str, region: &str) -> Result<Vec<u8>, PublishError> {
    let k_secret = format!("AWS4{secret_key}");
    let k_date = hmac_bytes(k_secret.as_bytes(), date)?;
    let k_region = hmac_bytes(&k_date, region)?;
    let k_service = hmac_bytes(&k_region, S3_SERVICE)?;
    hmac_bytes(&k_service, S3_REQUEST)
}

fn hmac_bytes(key: &[u8], message: &str) -> Result<Vec<u8>, PublishError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(to_publish_error)?;
    mac.update(message.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hmac_hex(key: &[u8], message: &str) -> Result<String, PublishError> {
    Ok(hex::encode(hmac_bytes(key, message)?))
}

fn parse_bool(value: &str) -> Result<bool, PublishError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(PublishError::General(format!(
            "Invalid boolean value: `{value}`"
        ))),
    }
}

fn file_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

fn to_publish_error(error: impl std::fmt::Display) -> PublishError {
    PublishError::General(error.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_object_key_without_prefix_uses_file_name() {
        let key = compose_object_key(None, "app.apk");
        assert_eq!(key, "app.apk");
    }

    #[test]
    fn compose_object_key_with_prefix_trims_slashes() {
        let key = compose_object_key(Some("/release/android/"), "app.apk");
        assert_eq!(key, "release/android/app.apk");
    }

    #[test]
    fn normalize_endpoint_adds_https_scheme() {
        let endpoint = normalize_endpoint("s3.us-east-1.amazonaws.com");
        assert_eq!(endpoint, "https://s3.us-east-1.amazonaws.com");
    }

    #[test]
    fn normalize_endpoint_keeps_existing_scheme() {
        let endpoint = normalize_endpoint("http://127.0.0.1:9000");
        assert_eq!(endpoint, "http://127.0.0.1:9000");
    }

    #[test]
    fn build_upload_url_supports_path_style() {
        let url = build_upload_url(
            "https://s3.us-east-1.amazonaws.com",
            true,
            "my-bucket",
            "release/app.apk",
        )
        .expect("path-style url should be built");
        assert_eq!(
            url,
            "https://s3.us-east-1.amazonaws.com/my-bucket/release/app.apk"
        );
    }

    #[test]
    fn build_upload_url_supports_virtual_host_style() {
        let url = build_upload_url(
            "https://s3.us-east-1.amazonaws.com",
            false,
            "my-bucket",
            "release/app.apk",
        )
        .expect("virtual-host-style url should be built");
        assert_eq!(
            url,
            "https://my-bucket.s3.us-east-1.amazonaws.com/release/app.apk"
        );
    }

    #[test]
    fn parse_bool_supports_common_values() {
        assert!(parse_bool("true").expect("true should parse"));
        assert!(parse_bool("1").expect("1 should parse"));
        assert!(!parse_bool("false").expect("false should parse"));
        assert!(!parse_bool("0").expect("0 should parse"));
    }

    #[test]
    fn parse_bool_rejects_invalid_value() {
        let error = parse_bool("maybe").expect_err("invalid bool should fail");
        assert_eq!(error.to_string(), "Invalid boolean value: `maybe`");
    }
}
