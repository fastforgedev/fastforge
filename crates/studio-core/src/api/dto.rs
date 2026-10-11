use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutPackagingFileRequest {
    pub content: String,
    /// Required optimistic concurrency token from GET, including for creation.
    pub etag: String,
}

/// `POST /v1/projects`
///
/// `local` mode registers an existing checkout, so it takes a `path`; `cloud`
/// mode creates a record, so it takes a `name`. One body covers both rather
/// than two endpoints, because the client is the same in both modes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Defaults to the directory name in local mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
}

/// `PATCH /v1/projects/{id}`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `POST /v1/projects/{id}/stores/{store}/apps`
///
/// Appends an entry to the store's `apps:` list in `.fastforge/config.yaml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStoreAppRequest {
    /// Bundle id (App Store) or package name (Google Play).
    pub identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
}

/// `PATCH /v1/projects/{id}/store-apps/{storeAppId}`
///
/// The identifier itself is not editable: it is the app's identity on the store
/// and the directory its catalog lives in. Changing it means removing the entry
/// and adding another.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStoreAppRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
}

/// `PUT /v1/projects/{id}/store-apps/{storeAppId}/catalog/file`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutCatalogFileRequest {
    pub content: String,
    /// The `etag` from the read. When present and stale, the write is rejected
    /// so a catalog refreshed by `pull` cannot be clobbered by an editor tab
    /// that has been open since before it ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_project_accepts_a_bare_path() {
        let request: CreateProjectRequest =
            serde_json::from_str(r#"{"path":"/Users/ada/app"}"#).unwrap();
        assert_eq!(request.path.as_deref(), Some("/Users/ada/app"));
        assert!(request.name.is_none());
    }

    #[test]
    fn store_app_fields_use_camel_case_on_the_wire() {
        let request: CreateStoreAppRequest =
            serde_json::from_str(r#"{"identifier":"com.example.app","appId":"123"}"#).unwrap();
        assert_eq!(request.app_id.as_deref(), Some("123"));
    }

    #[test]
    fn a_catalog_write_may_omit_its_etag() {
        let request: PutCatalogFileRequest =
            serde_json::from_str(r#"{"content":"name: App\n"}"#).unwrap();
        assert!(request.etag.is_none());
    }
}
