use serde::{Deserialize, Serialize};

/// Native packaging files supported by Studio's first set of forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackagingFormat {
    Deb,
    Pacman,
    Desktop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagingFile {
    /// Project-relative path; template variables are deliberately unexpanded.
    pub path: String,
    pub exists: bool,
    pub content: String,
    /// Includes the path and existence, so creation and renames also conflict.
    pub etag: String,
}
