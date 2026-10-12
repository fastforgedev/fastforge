use crate::common::{
    argument, artifact_path, file_name, file_part, http_client, required_env, to_publish_error,
};
use fastforge_app_analyzer::{AnalyzeConfig, AndroidApkAnalyzer, AppAnalyzer, IOSIpaAnalyzer};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use reqwest::blocking::Client;
use reqwest::blocking::multipart::Form;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

pub struct FirPublisher;

const PUBLISHER_NAME: &str = "fir";
const ENV_FIR_API_TOKEN: &str = "FIR_API_TOKEN";
const FIR_APPS_URL: &str = "http://api.bq04.com/apps";

#[derive(Debug, Deserialize)]
struct FirAppData {
    cert: FirCert,
    download_domain: String,
    download_domain_https_ready: bool,
    short: String,
}

#[derive(Debug, Deserialize)]
struct FirCert {
    binary: FirBinaryCert,
}

#[derive(Debug, Deserialize)]
struct FirBinaryCert {
    key: String,
    token: String,
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct FirUploadData {
    release_id: String,
}

#[derive(Debug, Deserialize)]
struct FirErrorBody {
    code: Option<i64>,
    errors: Option<FirErrors>,
}

#[derive(Debug, Deserialize)]
struct FirErrors {
    exception: Option<Vec<String>>,
}

/// What Dart's `parseAppPackage` returns for an APK/IPA.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct AppPackage {
    platform: String,
    identifier: String,
    name: String,
    version: String,
    build_number: String,
}

impl AppPublisher for FirPublisher {
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
        let api_token = required_env(config, ENV_FIR_API_TOKEN)?;
        let artifact_path = artifact_path(config)?;
        let app_package = resolve_app_package(config, artifact_path)?;

        let client = http_client()?;
        let app_data = get_upload_cert(&client, &api_token, &app_package)?;
        let release_id = upload_binary(
            &client,
            artifact_path,
            &app_data.cert.binary,
            &app_package,
            argument(config, &["changelog"]),
            on_progress,
        )?;

        let scheme = if app_data.download_domain_https_ready {
            "https"
        } else {
            "http"
        };
        let url = format!(
            "{}://{}/{}?release_id={}",
            scheme, app_data.download_domain, app_data.short, release_id
        );

        Ok(PublishResult {
            success: true,
            message: url,
        })
    }
}

/// Reads the app identity from the package like Dart's `parseAppPackage`.
/// Explicit `bundle_id` / `app_name` / `version` / `build_number` publish
/// arguments override the parsed values.
fn resolve_app_package(
    config: &PublishConfig,
    artifact_path: &str,
) -> Result<AppPackage, PublishError> {
    let overrides = AppPackage {
        platform: String::new(),
        identifier: argument(config, &["bundle_id", "bundle-id"]).unwrap_or_default(),
        name: argument(config, &["app_name", "app-name"]).unwrap_or_default(),
        version: argument(config, &["version"]).unwrap_or_default(),
        build_number: argument(config, &["build_number", "build-number"]).unwrap_or_default(),
    };
    let platform = infer_platform(artifact_path).ok_or_else(|| {
        PublishError::General(format!(
            "Cannot infer platform from artifact path: {artifact_path}"
        ))
    })?;

    let fully_overridden = !overrides.identifier.is_empty()
        && !overrides.name.is_empty()
        && !overrides.version.is_empty()
        && !overrides.build_number.is_empty();
    let parsed = if fully_overridden {
        AppPackage::default()
    } else {
        match parse_app_package(artifact_path, platform) {
            Ok(parsed) => parsed,
            Err(error) if !overrides.identifier.is_empty() => {
                eprintln!("Warning: failed to parse {artifact_path}: {error}");
                AppPackage::default()
            }
            Err(error) => return Err(error),
        }
    };

    let pick = |explicit: String, parsed: String| {
        if explicit.is_empty() {
            parsed
        } else {
            explicit
        }
    };
    Ok(AppPackage {
        platform: platform.to_string(),
        identifier: pick(overrides.identifier, parsed.identifier),
        name: pick(overrides.name, parsed.name),
        version: pick(overrides.version, parsed.version),
        build_number: pick(overrides.build_number, parsed.build_number),
    })
}

fn parse_app_package(artifact_path: &str, platform: &str) -> Result<AppPackage, PublishError> {
    let analyze_config = AnalyzeConfig::new(artifact_path.to_string());
    let result = match platform {
        "android" => AndroidApkAnalyzer::new().analyze(analyze_config),
        _ => IOSIpaAnalyzer::new().analyze(analyze_config),
    }
    .map_err(|error| {
        PublishError::General(format!(
            "Failed to parse app package {artifact_path}: {error}"
        ))
    })?;
    let package = app_package_from_analysis(platform, &result.data);
    if package.identifier.is_empty() {
        return Err(PublishError::General(format!(
            "Failed to read the bundle id of {artifact_path}"
        )));
    }
    Ok(package)
}

fn app_package_from_analysis(platform: &str, data: &Value) -> AppPackage {
    let text = |key: &str| match data.get(key) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        _ => String::new(),
    };
    AppPackage {
        platform: platform.to_string(),
        identifier: text("identifier"),
        name: text("name"),
        version: text("version"),
        build_number: text("buildNumber"),
    }
}

fn get_upload_cert(
    client: &Client,
    api_token: &str,
    app_package: &AppPackage,
) -> Result<FirAppData, PublishError> {
    let body = json!({
        "type": app_package.platform,
        "bundle_id": app_package.identifier,
        "api_token": api_token,
    });

    let response = client
        .post(FIR_APPS_URL)
        .json(&body)
        .send()
        .map_err(to_publish_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body: Option<FirErrorBody> = response.json().ok();
        let message = err_body
            .as_ref()
            .and_then(|b| {
                let code = b.code?;
                let msg = b.errors.as_ref()?.exception.as_ref()?.first()?;
                Some(format!("{code} - {msg}"))
            })
            .unwrap_or_else(|| format!("fir /apps request failed with status: {status}"));
        return Err(PublishError::General(message));
    }

    response.json::<FirAppData>().map_err(to_publish_error)
}

fn upload_binary(
    client: &Client,
    artifact_path: &str,
    cert: &FirBinaryCert,
    app_package: &AppPackage,
    changelog: Option<String>,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<String, PublishError> {
    let file_name = file_name(artifact_path)?;
    let mut form = Form::new()
        .text("key", cert.key.clone())
        .text("token", cert.token.clone())
        .text("x:name", app_package.name.clone())
        .text("x:version", app_package.version.clone())
        .text("x:build", app_package.build_number.clone());
    if let Some(changelog) = changelog {
        form = form.text("x:changelog", changelog);
    }
    let form = form.part("file", file_part(artifact_path, &file_name, on_progress)?);

    let response = client
        .post(&cert.upload_url)
        .multipart(form)
        .send()
        .map_err(to_publish_error)?;

    if !response.status().is_success() {
        return Err(PublishError::HttpError(format!(
            "fir upload failed with status: {}",
            response.status()
        )));
    }

    let data: FirUploadData = response.json().map_err(to_publish_error)?;
    Ok(data.release_id)
}

fn infer_platform(path: &str) -> Option<&'static str> {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("apk") => Some("android"),
        Some("ipa") => Some("ios"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_identity_from_analysis() {
        let data = json!({
            "identifier": "com.example.app",
            "name": "Example",
            "version": "1.2.3",
            "buildNumber": 45,
        });
        assert_eq!(
            app_package_from_analysis("android", &data),
            AppPackage {
                platform: "android".to_string(),
                identifier: "com.example.app".to_string(),
                name: "Example".to_string(),
                version: "1.2.3".to_string(),
                build_number: "45".to_string(),
            }
        );
    }

    #[test]
    fn explicit_arguments_skip_parsing() {
        let config = PublishConfig {
            publish_arguments: Some(
                [
                    ("bundle_id", "com.example.app"),
                    ("app_name", "Example"),
                    ("version", "1.0.0"),
                    ("build_number", "1"),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            ),
            ..Default::default()
        };
        // The file does not exist: parsing would fail, so this proves it is skipped.
        let package = resolve_app_package(&config, "/nonexistent/app.apk").unwrap();
        assert_eq!(package.platform, "android");
        assert_eq!(package.identifier, "com.example.app");
        assert_eq!(package.build_number, "1");
    }
}
