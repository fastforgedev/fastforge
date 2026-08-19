pub mod aab;
pub mod apk;

mod badging;
mod dependencies;
mod sdk;
mod signature;
mod techstack;

pub use aab::AndroidAabAnalyzer;
pub use apk::AndroidApkAnalyzer;

use crate::{checksum, json_util};
use serde_json::{Map, Value, json};
use std::path::Path;

/// File-level facts every Android artifact reports, alongside its identity.
fn artifact_fields(path: &Path) -> Map<String, Value> {
    let mut fields = Map::new();
    json_util::insert_text(
        &mut fields,
        "path",
        Some(
            std::fs::canonicalize(path)
                .unwrap_or_else(|_| path.to_path_buf())
                .to_string_lossy()
                .into_owned(),
        ),
    );
    json_util::insert_text(
        &mut fields,
        "fileName",
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned()),
    );
    fields.insert(
        "sizeBytes".to_string(),
        json!(std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)),
    );
    json_util::insert_text(&mut fields, "sha256", checksum::sha256_of(path));
    fields
}
