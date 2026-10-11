//! The OpenAPI document, compiled in.
//!
//! `apps/studio-api/openapi.yaml` is the single source of the contract: this crate
//! serves it, and `packages/studio-api-client` is generated from it. Embedding it here
//! means a host cannot ship a build whose documented contract has drifted from
//! the types it actually answers with — the tests below fail first.

use serde_json::Value;

use crate::api::ApiError;

const OPENAPI_YAML: &str = include_str!("../../../apps/studio-api/openapi.yaml");

const REFERENCE_SCRIPT_URL: &str = "https://cdn.jsdelivr.net/npm/@scalar/api-reference";

pub fn openapi_yaml() -> &'static str {
    OPENAPI_YAML
}

/// The document as JSON, for `GET /openapi.json`.
pub fn openapi_document() -> Result<Value, ApiError> {
    serde_yaml::from_str(OPENAPI_YAML).map_err(|error| {
        ApiError::internal(
            "INTERNAL_OPENAPI_ERROR",
            format!("Failed to parse openapi.yaml: {error}"),
        )
    })
}

/// A rendered reference page, for `GET /reference`.
pub fn reference_html(openapi_url: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>Fastforge Studio API</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <style>
      body {{ margin: 0; }}
    </style>
  </head>
  <body>
    <div id="app"></div>
    <script src="{REFERENCE_SCRIPT_URL}"></script>
    <script>
      Scalar.createApiReference('#app', {{ url: '{openapi_url}' }})
    </script>
  </body>
</html>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{StudioMode, capabilities_for};
    use crate::model::{CatalogEntryKind, Platform, RunKind, RunStatus, StoreKind};

    fn document() -> Value {
        openapi_document().expect("openapi.yaml must parse")
    }

    fn schema_enum(document: &Value, name: &str) -> Vec<String> {
        document["components"]["schemas"][name]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("schema {name} must declare an enum"))
            .iter()
            .map(|value| value.as_str().expect("enum values are strings").to_owned())
            .collect()
    }

    #[test]
    fn the_document_is_valid_yaml_with_the_expected_shape() {
        let document = document();
        assert_eq!(document["openapi"], "3.1.0");
        assert!(document["paths"]["/v1/capabilities"]["get"].is_object());
        assert!(document["paths"]["/v1/projects"]["get"].is_object());
    }

    // Each of these pins one Rust enum to its documented counterpart. Adding a
    // variant without touching openapi.yaml — or the reverse — fails here
    // rather than in a client that silently receives a value it cannot name.

    #[test]
    fn platform_matches_the_contract() {
        let documented = schema_enum(&document(), "Platform");
        let actual: Vec<String> = Platform::ALL
            .iter()
            .map(|platform| platform.as_str().to_owned())
            .collect();
        assert_eq!(documented, actual);
    }

    #[test]
    fn store_kind_matches_the_contract() {
        let documented = schema_enum(&document(), "StoreKind");
        let actual: Vec<String> = StoreKind::ALL
            .iter()
            .map(|store| store.as_str().to_owned())
            .collect();
        assert_eq!(documented, actual);
    }

    #[test]
    fn packaging_formats_match_the_contract() {
        use crate::model::PackagingFormat;
        let actual: Vec<String> = [
            PackagingFormat::Deb,
            PackagingFormat::Pacman,
            PackagingFormat::Desktop,
        ]
        .iter()
        .map(|format| {
            serde_json::to_value(format)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
        assert_eq!(schema_enum(&document(), "PackagingFormat"), actual);
    }

    #[test]
    fn run_kind_and_status_match_the_contract() {
        let document = document();
        assert_eq!(
            schema_enum(&document, "RunKind"),
            [
                RunKind::StoreCatalogPull.as_str(),
                RunKind::StoreCatalogPush.as_str()
            ]
        );
        assert_eq!(
            schema_enum(&document, "RunStatus"),
            [
                RunStatus::Queued.as_str(),
                RunStatus::Running.as_str(),
                RunStatus::Succeeded.as_str(),
                RunStatus::Failed.as_str(),
                RunStatus::Cancelled.as_str(),
            ]
        );
    }

    #[test]
    fn catalog_entry_kind_matches_the_contract() {
        let documented = schema_enum(&document(), "CatalogEntryKind");
        let actual: Vec<String> = [
            CatalogEntryKind::Dir,
            CatalogEntryKind::Yaml,
            CatalogEntryKind::Image,
            CatalogEntryKind::Video,
            CatalogEntryKind::Other,
        ]
        .iter()
        .map(|kind| {
            serde_json::to_value(kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
        assert_eq!(documented, actual);
    }

    #[test]
    fn every_capability_field_is_documented_and_required() {
        let document = document();
        let schema = &document["components"]["schemas"]["Capabilities"];
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();

        let serialized = serde_json::to_value(capabilities_for(StudioMode::Local)).unwrap();
        for key in serialized.as_object().unwrap().keys() {
            assert!(
                schema["properties"].get(key).is_some(),
                "capability `{key}` is missing from openapi.yaml"
            );
            assert!(
                required.contains(&key.as_str()),
                "capability `{key}` must be required — the client reads it unconditionally"
            );
        }
        assert_eq!(required.len(), serialized.as_object().unwrap().len());
    }

    #[test]
    fn the_reference_page_points_at_the_document() {
        let html = reference_html("/openapi.json");
        assert!(html.contains("url: '/openapi.json'"));
    }
}
