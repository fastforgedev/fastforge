use crate::common::{
    AppVersion, argument, artifact_path, file_body, file_name, http_client, required_env,
    resolve_app_version, to_publish_error,
};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

pub struct GitHubPublisher;

const PUBLISHER_NAME: &str = "github";
const ENV_GITHUB_TOKEN: &str = "GITHUB_TOKEN";
const ENV_GITHUB_REPOSITORY: &str = "GITHUB_REPOSITORY";
const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";
const GITHUB_API_VERSION: &str = "2022-11-28";

/// Characters Dart's `Uri.encodeComponent` leaves unescaped:
/// `A-Z a-z 0-9 - _ . ! ~ * ' ( )`.
const URI_COMPONENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

#[derive(Debug, Deserialize)]
struct Release {
    name: Option<String>,
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct AssetResponse {
    browser_download_url: Option<String>,
}

/// Mirrors Dart's `PublishGithubConfig`.
#[derive(Debug, PartialEq, Eq)]
struct GitHubConfig {
    token: String,
    repository: String,
    /// `None` publishes to the latest release.
    release_title: Option<String>,
    /// Rust extra: tag used when creating the release (defaults to the title).
    release_tag: Option<String>,
    release_draft: bool,
    release_prerelease: bool,
}

impl GitHubConfig {
    fn parse(config: &PublishConfig) -> Result<Self, PublishError> {
        let token = required_env(config, ENV_GITHUB_TOKEN)?;

        let mut repository = argument(config, &["repo", "github-repo"])
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                config
                    .env_var(ENV_GITHUB_REPOSITORY)
                    .filter(|value| !value.trim().is_empty())
            });
        // For backward compatibility, `repo-owner` and `repo-name` are supported.
        if repository.is_none() {
            let owner = argument(config, &["repo-owner", "github-repo-owner"])
                .filter(|value| !value.trim().is_empty());
            let name = argument(config, &["repo-name", "github-repo-name"])
                .filter(|value| !value.trim().is_empty());
            if let (Some(owner), Some(name)) = (owner, name) {
                repository = Some(format!("{owner}/{name}"));
            }
        }
        let repository = repository.ok_or_else(|| {
            PublishError::General(format!(
                "GitHub repository is required. Please provide it via `--github-repo` argument or `{ENV_GITHUB_REPOSITORY}` environment variable."
            ))
        })?;
        if !is_valid_repository(&repository) {
            return Err(PublishError::General(format!(
                "Invalid GitHub repository format. Expected format: `owner/repo` (e.g., `flutter/flutter`). Current value: `{repository}`"
            )));
        }

        let release_title = argument(config, &["release-title", "github-release-title"]);
        let release_title = match resolve_app_version(config)? {
            Some(version) => Some(release_title_for(release_title.as_deref(), &version)),
            // Dart leaves the title unset without a version (-> latest
            // release); an explicit title is kept as-is here instead.
            None => release_title.filter(|title| !title.trim().is_empty()),
        };

        Ok(Self {
            token,
            repository,
            release_title,
            release_tag: argument(config, &["release-tag", "github-release-tag"]),
            release_draft: flag(config, &["release-draft", "github-release-draft"]),
            release_prerelease: flag(config, &["release-prerelease", "github-release-prerelease"]),
        })
    }
}

/// Dart: `v${appVersion}` by default, otherwise `{appVersion}` /
/// `{appBuildName}` / `{appBuildNumber}` placeholders are substituted.
pub(crate) fn release_title_for(template: Option<&str>, version: &AppVersion) -> String {
    match template.filter(|title| !title.trim().is_empty()) {
        None => format!("v{}", version.text),
        Some(template) => template
            .replace("{appVersion}", &version.core)
            .replace("{appBuildName}", &version.core)
            .replace("{appBuildNumber}", &version.build),
    }
}

/// Dart's `^[^/]+/[^/]+$`.
pub(crate) fn is_valid_repository(repository: &str) -> bool {
    matches!(
        repository.split_once('/'),
        Some((owner, repo)) if !owner.is_empty() && !repo.is_empty() && !repo.contains('/')
    )
}

pub(crate) fn flag(config: &PublishConfig, keys: &[&str]) -> bool {
    argument(config, keys).is_some_and(|value| value == "true" || value == "1")
}

impl AppPublisher for GitHubPublisher {
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
        if !Path::new(artifact_path).is_file() {
            return Err(PublishError::General(format!(
                "The provided path \"{artifact_path}\" is not a valid file."
            )));
        }
        let file_name = file_name(artifact_path)?;
        let github_config = GitHubConfig::parse(config)?;

        let client = http_client()?;
        let upload_url = match &github_config.release_title {
            None => get_latest_release(&client, &github_config)?.upload_url,
            Some(title) => match find_release_by_name(&client, &github_config, title)? {
                Some(release) => release.upload_url,
                None => create_release(&client, &github_config, title)?.upload_url,
            },
        };
        if upload_url.is_empty() {
            return Err(PublishError::General("Upload url isEmpty".to_string()));
        }
        let download_url = upload_asset(
            &client,
            &github_config.token,
            &upload_url,
            artifact_path,
            &file_name,
            on_progress,
        )?;

        Ok(PublishResult {
            success: true,
            message: download_url,
        })
    }
}

fn github_request(builder: RequestBuilder, token: &str) -> RequestBuilder {
    builder
        .header(AUTHORIZATION, format!("token {token}"))
        .header(ACCEPT, GITHUB_ACCEPT)
        .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .header(USER_AGENT, "fastforge")
}

/// Dart's `_getUploadurlByReleaseName`: lists the releases (drafts included)
/// and matches on `name`.
fn find_release_by_name(
    client: &Client,
    config: &GitHubConfig,
    title: &str,
) -> Result<Option<Release>, PublishError> {
    let url = format!("{GITHUB_API_BASE}/repos/{}/releases", config.repository);
    let response = github_request(client.get(&url), &config.token)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        let text = response.text().unwrap_or_default();
        return Err(PublishError::HttpError(format!(
            "Failed to list GitHub releases: {text}"
        )));
    }
    let releases: Vec<Release> = response.json().map_err(to_publish_error)?;
    Ok(releases
        .into_iter()
        .find(|release| release.name.as_deref() == Some(title)))
}

fn create_release(
    client: &Client,
    config: &GitHubConfig,
    title: &str,
) -> Result<Release, PublishError> {
    let url = format!("{GITHUB_API_BASE}/repos/{}/releases", config.repository);
    let body = json!({
        "tag_name": config.release_tag.as_deref().unwrap_or(title),
        "name": title,
        "draft": config.release_draft,
        "prerelease": config.release_prerelease,
    });
    let response = github_request(client.post(&url), &config.token)
        .json(&body)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        let text = response.text().unwrap_or_default();
        return Err(PublishError::HttpError(format!(
            "Failed to create GitHub release: {text}"
        )));
    }
    response.json::<Release>().map_err(to_publish_error)
}

fn get_latest_release(client: &Client, config: &GitHubConfig) -> Result<Release, PublishError> {
    let url = format!(
        "{GITHUB_API_BASE}/repos/{}/releases/latest",
        config.repository
    );
    let response = github_request(client.get(&url), &config.token)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        let text = response.text().unwrap_or_default();
        return Err(PublishError::HttpError(format!(
            "Failed to get latest GitHub release: {text}"
        )));
    }
    response.json::<Release>().map_err(to_publish_error)
}

fn upload_asset(
    client: &Client,
    token: &str,
    upload_url: &str,
    artifact_path: &str,
    file_name: &str,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<String, PublishError> {
    let base_url = upload_url.split('{').next().unwrap_or(upload_url);
    let url = format!("{base_url}?name={}", encode_uri_component(file_name));
    let body = file_body(artifact_path, on_progress)?;

    let response = github_request(client.post(&url), token)
        .header(CONTENT_TYPE, "application/octet-stream")
        .body(body)
        .send()
        .map_err(to_publish_error)?;

    if !response.status().is_success() {
        let text = response.text().unwrap_or_default();
        return Err(PublishError::HttpError(format!(
            "GitHub asset upload failed: {text}"
        )));
    }

    let asset: AssetResponse = response.json().map_err(to_publish_error)?;
    asset
        .browser_download_url
        .filter(|url| !url.is_empty())
        .ok_or_else(|| PublishError::General(format!("Release asset exist [{file_name}]")))
}

pub(crate) fn encode_uri_component(value: &str) -> String {
    utf8_percent_encode(value, URI_COMPONENT_ENCODE_SET).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config_with(arguments: &[(&str, &str)]) -> PublishConfig {
        PublishConfig {
            app_version: Some("1.2.3+45".to_string()),
            publish_arguments: Some(
                arguments
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect::<HashMap<_, _>>(),
            ),
            environment: [(ENV_GITHUB_TOKEN.to_string(), "token".to_string())].into(),
            ..Default::default()
        }
    }

    #[test]
    fn default_title_is_v_plus_full_version() {
        let config = GitHubConfig::parse(&config_with(&[("repo", "owner/repo")])).unwrap();
        assert_eq!(config.release_title.as_deref(), Some("v1.2.3+45"));
        assert!(!config.release_draft);
        assert!(!config.release_prerelease);
    }

    #[test]
    fn title_template_is_substituted() {
        let config = GitHubConfig::parse(&config_with(&[
            ("repo", "owner/repo"),
            (
                "release-title",
                "v{appVersion} ({appBuildNumber}) {appBuildName}",
            ),
            ("release-draft", "true"),
        ]))
        .unwrap();
        assert_eq!(config.release_title.as_deref(), Some("v1.2.3 (45) 1.2.3"));
        assert!(config.release_draft);
    }

    #[test]
    fn legacy_owner_and_name_are_accepted() {
        let config = GitHubConfig::parse(&config_with(&[
            ("repo-owner", "leanflutter"),
            ("repo-name", "fastforge"),
        ]));
        // GITHUB_REPOSITORY from the process environment takes precedence.
        if std::env::var(ENV_GITHUB_REPOSITORY).is_ok_and(|v| !v.trim().is_empty()) {
            return;
        }
        assert_eq!(config.unwrap().repository, "leanflutter/fastforge");
    }

    #[test]
    fn invalid_repository_is_rejected() {
        let error = GitHubConfig::parse(&config_with(&[("repo", "owner/repo/extra")])).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid GitHub repository format. Expected format: `owner/repo` (e.g., `flutter/flutter`). Current value: `owner/repo/extra`"
        );
        assert!(!is_valid_repository("owner"));
        assert!(!is_valid_repository("/repo"));
        assert!(is_valid_repository("owner/repo"));
    }

    #[test]
    fn file_name_is_encoded_like_uri_encode_component() {
        assert_eq!(
            encode_uri_component("my app (1)+beta!~*'.apk"),
            "my%20app%20(1)%2Bbeta!~*'.apk"
        );
    }
}
