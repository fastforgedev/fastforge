//! Releases on Gitea and its forks Forgejo and Codeberg, which share
//! Gitea's `/api/v1` release API.

use crate::common::{
    argument, argument_or_env, artifact_path, file_name, file_part, http_client, required_env,
    resolve_app_version, to_publish_error,
};
use crate::github::{encode_uri_component, flag, is_valid_repository, release_title_for};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use reqwest::blocking::multipart::Form;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

pub struct GiteaPublisher;

const PUBLISHER_NAME: &str = "gitea";
const ENV_GITEA_TOKEN: &str = "GITEA_TOKEN";
const ENV_GITEA_SERVER_URL: &str = "GITEA_SERVER_URL";
const ENV_GITEA_REPOSITORY: &str = "GITEA_REPOSITORY";
/// Gitea and Forgejo Actions set the GitHub-compatible names.
const ENV_GITHUB_SERVER_URL: &str = "GITHUB_SERVER_URL";
const ENV_GITHUB_REPOSITORY: &str = "GITHUB_REPOSITORY";
/// Releases listed per page while looking one up by name.
const PAGE_SIZE: usize = 50;
const MAX_PAGES: usize = 20;

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssetResponse {
    browser_download_url: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct GiteaConfig {
    token: String,
    /// API root, such as `https://codeberg.org/api/v1`.
    api_base: String,
    repository: String,
    /// `None` publishes to the latest release.
    release_title: Option<String>,
    release_tag: Option<String>,
    release_draft: bool,
    release_prerelease: bool,
}

impl GiteaConfig {
    fn parse(config: &PublishConfig) -> Result<Self, PublishError> {
        let token = required_env(config, ENV_GITEA_TOKEN)?;
        let server = argument_or_env(
            config,
            &["server", "gitea-server"],
            &[ENV_GITEA_SERVER_URL, ENV_GITHUB_SERVER_URL],
        )
        .filter(|server| !server.trim().is_empty())
        .ok_or_else(|| {
            PublishError::General(format!(
                "Gitea server is required, such as `https://codeberg.org`. Please provide it via the `server` argument or `{ENV_GITEA_SERVER_URL}` environment variable."
            ))
        })?;
        if server.trim_end_matches('/') == "https://github.com" {
            return Err(PublishError::General(
                "The Gitea server is github.com; use the `github` target instead.".to_string(),
            ));
        }
        let repository = argument_or_env(
            config,
            &["repo", "gitea-repo"],
            &[ENV_GITEA_REPOSITORY, ENV_GITHUB_REPOSITORY],
        )
        .filter(|repo| !repo.trim().is_empty())
        .ok_or_else(|| {
            PublishError::General(format!(
                "Gitea repository is required. Please provide it via the `repo` argument or `{ENV_GITEA_REPOSITORY}` environment variable."
            ))
        })?;
        if !is_valid_repository(&repository) {
            return Err(PublishError::General(format!(
                "Invalid Gitea repository format. Expected format: `owner/repo`. Current value: `{repository}`"
            )));
        }

        let release_title = argument(config, &["release-title", "gitea-release-title"]);
        let release_title = match resolve_app_version(config)? {
            Some(version) => Some(release_title_for(release_title.as_deref(), &version)),
            None => release_title.filter(|title| !title.trim().is_empty()),
        };

        Ok(Self {
            token,
            api_base: api_base(&server),
            repository,
            release_title,
            release_tag: argument(config, &["release-tag", "gitea-release-tag"]),
            release_draft: flag(config, &["release-draft", "gitea-release-draft"]),
            release_prerelease: flag(config, &["release-prerelease", "gitea-release-prerelease"]),
        })
    }

    fn releases_url(&self) -> String {
        format!("{}/repos/{}/releases", self.api_base, self.repository)
    }
}

/// `https://codeberg.org` and `https://codeberg.org/api/v1/` both become
/// `https://codeberg.org/api/v1`.
fn api_base(server: &str) -> String {
    let server = server.trim().trim_end_matches('/');
    let server = server.strip_suffix("/api/v1").unwrap_or(server);
    format!("{server}/api/v1")
}

impl AppPublisher for GiteaPublisher {
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
        let gitea = GiteaConfig::parse(config)?;

        let client = http_client()?;
        let release = match &gitea.release_title {
            None => get_latest_release(&client, &gitea)?,
            Some(title) => match find_release_by_name(&client, &gitea, title)? {
                Some(release) => release,
                None => create_release(&client, &gitea, title)?,
            },
        };
        let download_url = upload_asset(
            &client,
            &gitea,
            release.id,
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

fn gitea_request(builder: RequestBuilder, token: &str) -> RequestBuilder {
    builder
        .header(AUTHORIZATION, format!("token {token}"))
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, "fastforge")
}

fn error_text(what: &str, response: reqwest::blocking::Response) -> PublishError {
    let status = response.status();
    let text = response.text().unwrap_or_default();
    PublishError::HttpError(format!("{what}: status={status} {text}"))
}

/// Lists the releases (drafts included) page by page and matches on `name`.
fn find_release_by_name(
    client: &Client,
    config: &GiteaConfig,
    title: &str,
) -> Result<Option<Release>, PublishError> {
    for page in 1..=MAX_PAGES {
        let response = gitea_request(client.get(config.releases_url()), &config.token)
            .query(&[("page", page), ("limit", PAGE_SIZE)])
            .send()
            .map_err(to_publish_error)?;
        if !response.status().is_success() {
            return Err(error_text("Failed to list Gitea releases", response));
        }
        let releases: Vec<Release> = response.json().map_err(to_publish_error)?;
        let last_page = releases.len() < PAGE_SIZE;
        if let Some(release) = releases
            .into_iter()
            .find(|release| release.name.as_deref() == Some(title))
        {
            return Ok(Some(release));
        }
        if last_page {
            break;
        }
    }
    Ok(None)
}

fn create_release(
    client: &Client,
    config: &GiteaConfig,
    title: &str,
) -> Result<Release, PublishError> {
    let body = json!({
        "tag_name": config.release_tag.as_deref().unwrap_or(title),
        "name": title,
        "draft": config.release_draft,
        "prerelease": config.release_prerelease,
    });
    let response = gitea_request(client.post(config.releases_url()), &config.token)
        .json(&body)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(error_text("Failed to create Gitea release", response));
    }
    response.json::<Release>().map_err(to_publish_error)
}

fn get_latest_release(client: &Client, config: &GiteaConfig) -> Result<Release, PublishError> {
    let url = format!("{}/latest", config.releases_url());
    let response = gitea_request(client.get(&url), &config.token)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(error_text("Failed to get latest Gitea release", response));
    }
    response.json::<Release>().map_err(to_publish_error)
}

fn upload_asset(
    client: &Client,
    config: &GiteaConfig,
    release_id: u64,
    artifact_path: &str,
    file_name: &str,
    on_progress: Option<&PublishProgressCallback>,
) -> Result<String, PublishError> {
    let url = format!(
        "{}/{release_id}/assets?name={}",
        config.releases_url(),
        encode_uri_component(file_name)
    );
    let form = Form::new().part(
        "attachment",
        file_part(artifact_path, file_name, on_progress)?,
    );
    let response = gitea_request(client.post(&url), &config.token)
        .multipart(form)
        .send()
        .map_err(to_publish_error)?;
    if !response.status().is_success() {
        return Err(error_text("Gitea asset upload failed", response));
    }
    let asset: AssetResponse = response.json().map_err(to_publish_error)?;
    asset
        .browser_download_url
        .filter(|url| !url.is_empty())
        .ok_or_else(|| PublishError::General(format!("Gitea returned no URL for [{file_name}]")))
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
            environment: [(ENV_GITEA_TOKEN.to_string(), "token".to_string())].into(),
            ..Default::default()
        }
    }

    #[test]
    fn server_becomes_the_api_root() {
        assert_eq!(
            api_base("https://codeberg.org"),
            "https://codeberg.org/api/v1"
        );
        assert_eq!(
            api_base(" https://git.example.com/gitea/api/v1/ "),
            "https://git.example.com/gitea/api/v1"
        );
    }

    #[test]
    fn parses_server_repository_and_release() {
        let config = GiteaConfig::parse(&config_with(&[
            ("server", "https://codeberg.org/"),
            ("repo", "owner/app"),
            ("release-title", "App {appBuildName}"),
            ("release-prerelease", "true"),
        ]))
        .unwrap();
        assert_eq!(config.api_base, "https://codeberg.org/api/v1");
        assert_eq!(
            config.releases_url(),
            "https://codeberg.org/api/v1/repos/owner/app/releases"
        );
        assert_eq!(config.release_title.as_deref(), Some("App 1.2.3"));
        assert!(config.release_prerelease && !config.release_draft);
    }

    #[test]
    fn rejects_github_and_bad_repositories() {
        let github = GiteaConfig::parse(&config_with(&[
            ("server", "https://github.com"),
            ("repo", "owner/app"),
        ]));
        assert!(github.unwrap_err().to_string().contains("`github` target"));
        let invalid = GiteaConfig::parse(&config_with(&[
            ("server", "https://codeberg.org"),
            ("repo", "owner"),
        ]));
        assert!(invalid.unwrap_err().to_string().contains("owner/repo"));
    }
}
