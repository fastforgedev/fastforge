pub mod distribute_options;
pub mod release;

use anyhow::{Context, Result};
pub use distribute_options::DistributeOptions;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parsed contents of `.fastforge/config.yaml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Distribution store definitions.
    #[serde(default)]
    pub stores: StoresConfig,
}

impl Config {
    pub const DEFAULT_PATH: &'static str = ".fastforge/config.yaml";

    /// Load and parse `.fastforge/config.yaml` from `path`.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let mut config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        config.resolve_env_refs();
        Ok(config)
    }

    /// Load `.fastforge/config.yaml` from the current working directory,
    /// returning an empty config when the file does not exist.
    pub fn load() -> Result<Self> {
        let path = Path::new(Self::DEFAULT_PATH);
        if path.exists() {
            Self::from_file(path)
        } else {
            Ok(Self::default())
        }
    }

    fn resolve_env_refs(&mut self) {
        if let Some(appstore) = self.stores.appstore.as_mut() {
            appstore.resolve_env_refs();
        }
        if let Some(appgallery) = self.stores.appgallery.as_mut() {
            appgallery.resolve_env_refs();
        }
        if let Some(googleplay) = self.stores.googleplay.as_mut() {
            googleplay.resolve_env_refs();
        }
    }
}

/// Supported distribution stores.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoresConfig {
    #[serde(default)]
    pub appstore: Option<AppStoreConfig>,
    #[serde(default)]
    pub appgallery: Option<AppGalleryConfig>,
    #[serde(default)]
    pub googleplay: Option<GooglePlayConfig>,
}

impl StoresConfig {
    pub fn is_empty(&self) -> bool {
        self.appstore.is_none() && self.appgallery.is_none() && self.googleplay.is_none()
    }
}

/// App Store Connect store configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStoreConfig {
    #[serde(default)]
    pub auth: AppStoreAuthConfig,
    #[serde(default)]
    pub apps: Vec<AppStoreApp>,
}

impl AppStoreConfig {
    fn resolve_env_refs(&mut self) {
        self.auth.resolve_env_refs();
        self.auth.apply_env_defaults();
    }
}

/// App Store Connect authentication configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppStoreAuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl AppStoreAuthConfig {
    pub fn auth_type(&self) -> &'static str {
        if self.key_id.is_some() || self.issuer_id.is_some() || self.key_path.is_some() {
            "api_key"
        } else if self.username.is_some() || self.password.is_some() {
            "username_password"
        } else {
            "unknown"
        }
    }

    fn resolve_env_refs(&mut self) {
        resolve_optional_env_ref(&mut self.key_id);
        resolve_optional_env_ref(&mut self.issuer_id);
        resolve_optional_env_ref(&mut self.key_path);
        resolve_optional_env_ref(&mut self.username);
        resolve_optional_env_ref(&mut self.password);
    }

    fn apply_env_defaults(&mut self) {
        set_from_env_if_empty(
            &mut self.key_id,
            &["APP_STORE_CONNECT_KEY_ID", "APPSTORE_APIKEY"],
        );
        set_from_env_if_empty(
            &mut self.issuer_id,
            &["APP_STORE_CONNECT_ISSUER_ID", "APPSTORE_APIISSUER"],
        );
        set_from_env_if_empty(&mut self.key_path, &["APP_STORE_CONNECT_KEY_PATH"]);
        set_from_env_if_empty(&mut self.username, &["APPSTORE_USERNAME"]);
        set_from_env_if_empty(&mut self.password, &["APPSTORE_PASSWORD"]);
    }
}

/// App metadata recorded under App Store Connect.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppStoreApp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl AppStoreApp {
    pub fn identifier(&self) -> Option<&str> {
        self.bundle_id.as_deref()
    }
}

/// Huawei AppGallery Connect store configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppGalleryConfig {
    #[serde(default)]
    pub auth: AppGalleryAuthConfig,
    #[serde(default)]
    pub apps: Vec<AppGalleryApp>,
}

impl AppGalleryConfig {
    fn resolve_env_refs(&mut self) {
        self.auth.resolve_env_refs();
        self.auth.apply_env_defaults();
    }
}

/// AppGallery Connect authentication configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppGalleryAuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_account_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
}

impl AppGalleryAuthConfig {
    pub fn auth_type(&self) -> &'static str {
        if self.service_account_key.is_some() || self.service_account_json.is_some() {
            "service_account"
        } else if self.client_id.is_some() || self.client_secret.is_some() {
            "api_client"
        } else {
            "unknown"
        }
    }

    fn resolve_env_refs(&mut self) {
        resolve_optional_env_ref(&mut self.service_account_key);
        resolve_optional_env_ref(&mut self.service_account_json);
        resolve_optional_env_ref(&mut self.client_id);
        resolve_optional_env_ref(&mut self.client_secret);
    }

    fn apply_env_defaults(&mut self) {
        set_from_env_if_empty(
            &mut self.service_account_key,
            &["APP_GALLERY_SERVICE_ACCOUNT_KEY"],
        );
        set_from_env_if_empty(
            &mut self.service_account_json,
            &["APP_GALLERY_SERVICE_ACCOUNT_JSON"],
        );
        set_from_env_if_empty(&mut self.client_id, &["APP_GALLERY_CLIENT_ID"]);
        set_from_env_if_empty(&mut self.client_secret, &["APP_GALLERY_CLIENT_SECRET"]);
    }
}

/// App metadata recorded under AppGallery Connect.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppGalleryApp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl AppGalleryApp {
    pub fn identifier(&self) -> Option<&str> {
        self.app_id.as_deref().or(self.package_name.as_deref())
    }
}

/// Google Play Console store configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooglePlayConfig {
    #[serde(default)]
    pub auth: GooglePlayAuthConfig,
    #[serde(default)]
    pub apps: Vec<GooglePlayApp>,
}

impl GooglePlayConfig {
    fn resolve_env_refs(&mut self) {
        self.auth.resolve_env_refs();
        self.auth.apply_env_defaults();
    }
}

/// Google Play Console authentication configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GooglePlayAuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_account_json: Option<String>,
}

impl GooglePlayAuthConfig {
    pub fn auth_type(&self) -> &'static str {
        if self.service_account_key.is_some() || self.service_account_json.is_some() {
            "service_account"
        } else {
            "unknown"
        }
    }

    fn resolve_env_refs(&mut self) {
        resolve_optional_env_ref(&mut self.service_account_key);
        resolve_optional_env_ref(&mut self.service_account_json);
    }

    fn apply_env_defaults(&mut self) {
        set_from_env_if_empty(
            &mut self.service_account_key,
            &[
                "GOOGLE_PLAY_SERVICE_ACCOUNT_KEY",
                "GOOGLE_APPLICATION_CREDENTIALS",
            ],
        );
        set_from_env_if_empty(
            &mut self.service_account_json,
            &["GOOGLE_PLAY_SERVICE_ACCOUNT_JSON"],
        );
    }
}

/// App metadata recorded under Google Play Console.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GooglePlayApp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
}

impl GooglePlayApp {
    pub fn identifier(&self) -> Option<&str> {
        self.package_name.as_deref()
    }
}

fn resolve_optional_env_ref(value: &mut Option<String>) {
    if let Some(current) = value
        && let Some(resolved) = resolve_env_ref(current)
    {
        *current = resolved;
    }
}

fn set_from_env_if_empty(value: &mut Option<String>, env_keys: &[&str]) {
    if value.as_ref().is_some_and(|value| !value.trim().is_empty()) {
        return;
    }

    if let Some(env_value) = env_keys.iter().find_map(|key| std::env::var(key).ok()) {
        *value = Some(env_value);
    }
}

fn resolve_env_ref(value: &str) -> Option<String> {
    let name = value.strip_prefix("${")?.strip_suffix('}')?;
    if name.is_empty() {
        return None;
    }
    std::env::var(name).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const STORES_YAML: &str = r#"
stores:
  appstore:
    auth:
      key_id: configured-key-id
      issuer_id: issuer
      key_path: ./AuthKey.p8
    apps:
      - bundle_id: com.example.myapp
        app_id: "1234567890"
        sku: MYAPP001
        name: My App
  appgallery:
    auth:
      service_account_key: ./appgallery-private.json
    apps:
      - app_id: "123456789"
        package_name: com.example.myapp
        name: My App
  googleplay:
    auth:
      service_account_key: ./service-account.json
    apps:
      - package_name: com.example.myapp
        track: production
"#;

    fn parse_yaml(content: &str) -> Config {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        Config::from_file(file.path()).unwrap()
    }

    #[test]
    fn test_parse_stores() {
        let config = parse_yaml(STORES_YAML);
        assert!(!config.stores.is_empty());

        let appstore = config.stores.appstore.as_ref().unwrap();
        assert_eq!(appstore.auth.auth_type(), "api_key");
        assert_eq!(appstore.auth.issuer_id.as_deref(), Some("issuer"));
        assert_eq!(appstore.apps[0].identifier(), Some("com.example.myapp"));

        let appgallery = config.stores.appgallery.as_ref().unwrap();
        assert_eq!(appgallery.auth.auth_type(), "service_account");
        assert_eq!(appgallery.apps[0].identifier(), Some("123456789"));

        let googleplay = config.stores.googleplay.as_ref().unwrap();
        assert_eq!(googleplay.auth.auth_type(), "service_account");
        assert_eq!(googleplay.apps[0].identifier(), Some("com.example.myapp"));
        assert_eq!(googleplay.apps[0].track.as_deref(), Some("production"));
    }

    #[test]
    fn test_missing_env_ref_is_preserved() {
        let config = parse_yaml(
            r#"
stores:
  appstore:
    auth:
      key_id: ${FASTFORGE_TEST_ENV_SHOULD_NOT_EXIST}
"#,
        );
        let appstore = config.stores.appstore.as_ref().unwrap();
        assert_eq!(
            appstore.auth.key_id.as_deref(),
            Some("${FASTFORGE_TEST_ENV_SHOULD_NOT_EXIST}")
        );
    }

    #[test]
    fn test_default_config_is_empty() {
        let config = Config::default();
        assert!(config.stores.is_empty());
    }

    #[test]
    fn test_auth_can_be_omitted() {
        let config = parse_yaml(
            r#"
stores:
  appstore:
    apps:
      - bundle_id: com.example.myapp
  appgallery:
    apps:
      - app_id: "123456789"
  googleplay:
    apps:
      - package_name: com.example.myapp
"#,
        );

        assert_eq!(
            config.stores.appstore.as_ref().unwrap().apps[0].identifier(),
            Some("com.example.myapp")
        );
        assert_eq!(
            config.stores.appgallery.as_ref().unwrap().apps[0].identifier(),
            Some("123456789")
        );
        assert_eq!(
            config.stores.googleplay.as_ref().unwrap().apps[0].identifier(),
            Some("com.example.myapp")
        );
    }
}
