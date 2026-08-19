use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::fs;

pub const SERVICE_ACCOUNT_ENV: &str = "APP_GALLERY_SERVICE_ACCOUNT_JSON";
pub const SERVICE_ACCOUNT_KEY_ENV: &str = "APP_GALLERY_SERVICE_ACCOUNT_KEY";
pub const CLIENT_ID_ENV: &str = "APP_GALLERY_CLIENT_ID";
pub const CLIENT_SECRET_ENV: &str = "APP_GALLERY_CLIENT_SECRET";
pub const API_BASE: &str = "https://connect-api.cloud.huawei.com";
pub const LEGACY_TOKEN_URI: &str = "https://connect-api.cloud.huawei.com/api/oauth2/v1/token";
pub const SERVICE_ACCOUNT_TOKEN_URI: &str = "https://oauth-login.cloud.huawei.com/oauth2/v3/token";

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountCredentials {
    pub key_id: String,
    pub private_key: String,
    pub sub_account: String,
    #[serde(default)]
    pub token_uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiClientCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone)]
pub enum AppGalleryAuth {
    ServiceAccount(ServiceAccountCredentials),
    ApiClient(ApiClientCredentials),
}

#[derive(Debug, Serialize)]
struct ServiceAccountClaims<'a> {
    aud: &'a str,
    iss: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Serialize)]
struct LegacyTokenRequest<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    client_secret: &'a str,
}

#[derive(Debug, Deserialize)]
struct LegacyTokenResponse {
    access_token: String,
}

impl AppGalleryAuth {
    pub fn from_env() -> Result<Self> {
        if let Some(value) = [SERVICE_ACCOUNT_ENV, SERVICE_ACCOUNT_KEY_ENV]
            .iter()
            .find_map(|name| {
                std::env::var(name)
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
        {
            return Ok(Self::ServiceAccount(parse_service_account(&value)?));
        }

        let client_id = required_env(CLIENT_ID_ENV)?;
        let client_secret = required_env(CLIENT_SECRET_ENV)?;
        Ok(Self::ApiClient(ApiClientCredentials {
            client_id,
            client_secret,
        }))
    }

    pub fn auth_type(&self) -> &'static str {
        match self {
            Self::ServiceAccount(_) => "service_account",
            Self::ApiClient(_) => "api_client",
        }
    }

    pub fn client_id(&self) -> Option<&str> {
        match self {
            Self::ServiceAccount(_) => None,
            Self::ApiClient(credentials) => Some(&credentials.client_id),
        }
    }

    pub async fn access_token(&self) -> Result<String> {
        match self {
            Self::ServiceAccount(credentials) => service_account_token(credentials),
            Self::ApiClient(credentials) => api_client_token(credentials).await,
        }
    }
}

fn service_account_token(credentials: &ServiceAccountCredentials) -> Result<String> {
    let audience = credentials
        .token_uri
        .as_deref()
        .unwrap_or(SERVICE_ACCOUNT_TOKEN_URI);
    let issued_at = Utc::now().timestamp();
    let claims = ServiceAccountClaims {
        aud: audience,
        iss: &credentials.sub_account,
        iat: issued_at,
        exp: issued_at + 3600,
    };
    let mut header = Header::new(Algorithm::PS256);
    header.kid = Some(credentials.key_id.clone());
    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(credentials.private_key.as_bytes())
            .context("failed to parse AppGallery service-account private key")?,
    )
    .context("failed to sign AppGallery service-account JWT")
}

async fn api_client_token(credentials: &ApiClientCredentials) -> Result<String> {
    let response = reqwest::Client::new()
        .post(LEGACY_TOKEN_URI)
        .json(&LegacyTokenRequest {
            grant_type: "client_credentials",
            client_id: &credentials.client_id,
            client_secret: &credentials.client_secret,
        })
        .send()
        .await
        .context("failed to request AppGallery API client token")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "failed to get AppGallery API client token: {status}\n{body}"
        ));
    }
    let body: LegacyTokenResponse = response
        .json()
        .await
        .context("failed to decode AppGallery API client token response")?;
    Ok(body.access_token)
}

fn parse_service_account(value: &str) -> Result<ServiceAccountCredentials> {
    match serde_json::from_str(value) {
        Ok(credentials) => Ok(credentials),
        Err(json_error) => {
            let content = fs::read_to_string(value).with_context(|| {
                format!(
                    "{SERVICE_ACCOUNT_ENV} must contain service-account JSON or a path to a JSON file; JSON parse error: {json_error}"
                )
            })?;
            serde_json::from_str(&content).with_context(|| {
                format!("failed to parse AppGallery service-account JSON file: {value}")
            })
        }
    }
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "missing AppGallery credentials: set {SERVICE_ACCOUNT_ENV} (or {SERVICE_ACCOUNT_KEY_ENV}), or both {CLIENT_ID_ENV} and {CLIENT_SECRET_ENV}"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_service_account_json() {
        let credentials = parse_service_account(
            r#"{"key_id":"key","private_key":"pem","sub_account":"account"}"#,
        )
        .unwrap();
        assert_eq!(credentials.key_id, "key");
        assert_eq!(credentials.sub_account, "account");
        assert!(credentials.token_uri.is_none());
    }
}
