use crate::auth::{API_BASE, AppGalleryAuth};
use anyhow::{Context as _, Result};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};

#[derive(Clone)]
pub struct AppGalleryContext {
    pub api_base: &'static str,
    pub auth: AppGalleryAuth,
    pub http: reqwest::Client,
    pub client: crate::Client,
}

impl AppGalleryContext {
    pub async fn from_env() -> Result<Self> {
        Self::new(AppGalleryAuth::from_env()?).await
    }

    pub async fn new(auth: AppGalleryAuth) -> Result<Self> {
        let access_token = auth.access_token().await?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {access_token}"))
                .context("failed to build AppGallery Authorization header")?,
        );
        if let Some(client_id) = auth.client_id() {
            headers.insert(
                HeaderName::from_static("client_id"),
                HeaderValue::from_str(client_id)
                    .context("failed to build AppGallery client_id header")?,
            );
        }
        let http = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .context("failed to build AppGallery HTTP client")?;
        let client = crate::Client::new_with_client(API_BASE, http.clone());
        Ok(Self {
            api_base: API_BASE,
            auth,
            http,
            client,
        })
    }

    pub fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.api_base, path)
    }
}
