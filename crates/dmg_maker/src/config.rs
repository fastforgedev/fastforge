use crate::error::DmgMakerError;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ParsedSpec {
    pub spec: AppDmgSpec,
    pub resolve_base: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub source: Option<PathBuf>,
    pub specification: Option<Value>,
    pub basepath: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppDmgSpec {
    pub title: String,
    pub icon: Option<String>,
    pub background: Option<String>,
    #[serde(rename = "background-color")]
    pub background_color: Option<String>,
    #[serde(
        default,
        rename = "icon-size",
        deserialize_with = "deserialize_opt_u32_from_num"
    )]
    pub icon_size: Option<u32>,
    pub window: Option<Window>,
    pub format: Option<String>,
    pub filesystem: Option<String>,
    pub contents: Vec<ContentEntry>,
    #[serde(rename = "code-sign")]
    pub code_sign: Option<CodeSign>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Window {
    pub position: Option<Position>,
    pub size: Option<WindowSize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Position {
    #[serde(deserialize_with = "deserialize_i32_from_num")]
    pub x: i32,
    #[serde(deserialize_with = "deserialize_i32_from_num")]
    pub y: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WindowSize {
    #[serde(deserialize_with = "deserialize_i32_from_num")]
    pub width: i32,
    #[serde(deserialize_with = "deserialize_i32_from_num")]
    pub height: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Link,
    File,
    Position,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentEntry {
    #[serde(deserialize_with = "deserialize_i32_from_num")]
    pub x: i32,
    #[serde(deserialize_with = "deserialize_i32_from_num")]
    pub y: i32,
    #[serde(rename = "type")]
    pub kind: ContentType,
    pub path: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeSign {
    #[serde(rename = "signing-identity")]
    pub signing_identity: String,
    pub identifier: Option<String>,
    /// Adds a secure timestamp (`codesign --timestamp`), which notarization
    /// requires.
    pub timestamp: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacySpec {
    title: String,
    app: String,
    icon: Option<String>,
    background: Option<String>,
    icons: LegacyIcons,
    extra: Option<Vec<LegacyExtra>>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyIcons {
    #[serde(deserialize_with = "deserialize_u32_from_num")]
    size: u32,
    app: [NumI32; 2],
    alias: [NumI32; 2],
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(transparent)]
struct NumI32(#[serde(deserialize_with = "deserialize_i32_from_num")] i32);

type LegacyExtra = (String, NumI32, NumI32);

fn deserialize_i32_from_num<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct NumVisitor;

    impl serde::de::Visitor<'_> for NumVisitor {
        type Value = i32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an integer or floating-point number")
        }

        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
            i32::try_from(v).map_err(|_| E::custom(format!("number out of i32 range: {v}")))
        }

        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
            i32::try_from(v).map_err(|_| E::custom(format!("number out of i32 range: {v}")))
        }

        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
            if !v.is_finite() {
                return Err(E::custom(format!("non-finite number: {v}")));
            }
            let rounded = v.round();
            if rounded < i32::MIN as f64 || rounded > i32::MAX as f64 {
                return Err(E::custom(format!("number out of i32 range: {v}")));
            }
            Ok(rounded as i32)
        }
    }

    deserializer.deserialize_any(NumVisitor)
}

fn deserialize_u32_from_num<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct NumVisitor;

    impl serde::de::Visitor<'_> for NumVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a non-negative integer or floating-point number")
        }

        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
            u32::try_from(v).map_err(|_| E::custom(format!("number out of u32 range: {v}")))
        }

        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
            u32::try_from(v).map_err(|_| E::custom(format!("number out of u32 range: {v}")))
        }

        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
            if !v.is_finite() {
                return Err(E::custom(format!("non-finite number: {v}")));
            }
            let rounded = v.round();
            if rounded < 0.0 || rounded > u32::MAX as f64 {
                return Err(E::custom(format!("number out of u32 range: {v}")));
            }
            Ok(rounded as u32)
        }
    }

    deserializer.deserialize_any(NumVisitor)
}

fn deserialize_opt_u32_from_num<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(transparent)]
    struct NumU32(#[serde(deserialize_with = "deserialize_u32_from_num")] u32);

    Ok(Option::<NumU32>::deserialize(deserializer)?.map(|n| n.0))
}

impl AppDmgSpec {
    pub fn validate(&self) -> Result<(), DmgMakerError> {
        if self.title.trim().is_empty() {
            return Err(DmgMakerError::InvalidConfig(
                "`title` must not be empty".to_string(),
            ));
        }

        if self.contents.is_empty() {
            return Err(DmgMakerError::InvalidConfig(
                "`contents` must not be empty".to_string(),
            ));
        }

        if let Some(format) = &self.format {
            let allowed = ["UDRW", "UDRO", "UDCO", "UDZO", "ULFO", "ULMO", "UDBZ"];
            if !allowed.contains(&format.as_str()) {
                return Err(DmgMakerError::InvalidConfig(format!(
                    "Invalid `format`: {format}"
                )));
            }
        }

        if let Some(filesystem) = &self.filesystem {
            let allowed = ["HFS+", "APFS"];
            if !allowed.contains(&filesystem.as_str()) {
                return Err(DmgMakerError::InvalidConfig(format!(
                    "Invalid `filesystem`: {filesystem}"
                )));
            }
        }

        Ok(())
    }
}

pub fn load_spec(options: &LoadOptions) -> Result<ParsedSpec, DmgMakerError> {
    let has_source = options.source.is_some();
    let has_spec = options.basepath.is_some() && options.specification.is_some();

    if has_source == has_spec {
        return Err(DmgMakerError::InvalidConfig(
            "Supply one of `source` or `(basepath, specification)`".to_string(),
        ));
    }

    if let Some(source) = &options.source {
        let raw = std::fs::read_to_string(source).map_err(|e| {
            DmgMakerError::General(format!(
                "JSON Specification not found at: {}: {}",
                source.display(),
                e
            ))
        })?;
        let value: Value = serde_json::from_str(&raw).map_err(DmgMakerError::Json)?;
        let spec = parse_spec_value(value)?;

        return Ok(ParsedSpec {
            spec,
            resolve_base: source
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
        });
    }

    let spec = parse_spec_value(
        options
            .specification
            .clone()
            .ok_or_else(|| DmgMakerError::InvalidConfig("Missing `specification`".to_string()))?,
    )?;

    Ok(ParsedSpec {
        spec,
        resolve_base: options
            .basepath
            .clone()
            .ok_or_else(|| DmgMakerError::InvalidConfig("Missing `basepath`".to_string()))?,
    })
}

fn parse_spec_value(value: Value) -> Result<AppDmgSpec, DmgMakerError> {
    let spec = if value.get("icons").is_some() {
        let legacy: LegacySpec = serde_json::from_value(value).map_err(DmgMakerError::Json)?;
        convert_legacy(legacy)
    } else {
        serde_json::from_value(value).map_err(DmgMakerError::Json)?
    };

    spec.validate()?;
    Ok(spec)
}

fn convert_legacy(src: LegacySpec) -> AppDmgSpec {
    let mut contents = vec![
        ContentEntry {
            x: src.icons.alias[0].0,
            y: src.icons.alias[1].0,
            kind: ContentType::Link,
            path: "/Applications".to_string(),
            name: None,
        },
        ContentEntry {
            x: src.icons.app[0].0,
            y: src.icons.app[1].0,
            kind: ContentType::File,
            path: src.app,
            name: None,
        },
    ];

    if let Some(extra) = src.extra {
        for item in extra {
            contents.push(ContentEntry {
                x: item.1.0,
                y: item.2.0,
                kind: ContentType::File,
                path: item.0,
                name: None,
            });
        }
    }

    AppDmgSpec {
        title: src.title,
        icon: src.icon,
        background: src.background,
        background_color: None,
        icon_size: Some(src.icons.size),
        window: None,
        format: None,
        filesystem: None,
        contents,
        code_sign: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentType, LoadOptions, load_spec};
    use serde_json::json;
    use std::path::PathBuf;

    fn examples_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
    }

    fn load_example(name: &str) -> super::ParsedSpec {
        load_spec(&LoadOptions {
            source: Some(examples_dir().join(name)),
            basepath: None,
            specification: None,
        })
        .expect("parse should succeed")
    }

    #[test]
    fn parse_modern_spec() {
        let parsed = load_spec(&LoadOptions {
            source: None,
            basepath: Some(PathBuf::from(".")),
            specification: Some(json!({
                "title": "Test App",
                "contents": [
                    { "x": 1, "y": 2, "type": "link", "path": "/Applications" },
                    { "x": 3, "y": 4, "type": "file", "path": "Test.app" }
                ]
            })),
        })
        .expect("parse should succeed");

        assert_eq!(parsed.spec.title, "Test App");
        assert_eq!(parsed.spec.contents.len(), 2);
        assert_eq!(parsed.spec.contents[0].kind, ContentType::Link);
    }

    #[test]
    fn parse_legacy_spec() {
        let parsed = load_spec(&LoadOptions {
            source: None,
            basepath: Some(PathBuf::from(".")),
            specification: Some(json!({
                "title": "Legacy App",
                "app": "Legacy.app",
                "icons": {
                    "size": 80,
                    "app": [192, 344],
                    "alias": [448, 344]
                },
                "extra": [["Readme.txt", 100, 100]]
            })),
        })
        .expect("parse should succeed");

        assert_eq!(parsed.spec.title, "Legacy App");
        assert_eq!(parsed.spec.icon_size, Some(80));
        assert_eq!(parsed.spec.contents.len(), 3);
        assert_eq!(parsed.spec.contents[0].kind, ContentType::Link);
        assert_eq!(parsed.spec.contents[1].kind, ContentType::File);
    }

    #[test]
    fn parse_standard_spec_from_file_example() {
        let parsed = load_example("standard.json");

        assert_eq!(parsed.spec.title, "Sample App");
        assert_eq!(
            parsed.resolve_base,
            examples_dir(),
            "relative paths should resolve from examples dir"
        );
        assert_eq!(parsed.spec.icon.as_deref(), Some("assets/SampleIcon.icns"));
        assert_eq!(
            parsed.spec.background.as_deref(),
            Some("assets/SampleBackground.png")
        );
        assert_eq!(parsed.spec.contents.len(), 4);
        assert_eq!(parsed.spec.contents[0].kind, ContentType::Link);
        assert_eq!(parsed.spec.contents[1].kind, ContentType::File);
        assert_eq!(parsed.spec.contents[3].kind, ContentType::Position);
    }

    #[test]
    fn parse_compat_legacy_spec_from_file_example() {
        let parsed = load_example("compat_legacy.json");

        assert_eq!(parsed.spec.title, "Sample App");
        assert_eq!(
            parsed.spec.background.as_deref(),
            Some("assets/SampleBackground.png")
        );
        assert_eq!(parsed.spec.icon_size, Some(80));
        assert_eq!(parsed.spec.contents.len(), 3);
        assert_eq!(parsed.spec.contents[0].kind, ContentType::Link);
        assert_eq!(parsed.spec.contents[1].path, "assets/SampleApp.app");
        assert_eq!(parsed.spec.contents[2].path, "assets/Readme.txt");
    }

    #[test]
    fn parse_background_color_spec_from_file_example() {
        let parsed = load_example("background_color.json");

        assert_eq!(parsed.spec.title, "Sample App");
        assert_eq!(parsed.spec.background_color.as_deref(), Some("mintcream"));
    }

    #[test]
    fn parse_window_spec_from_file_example() {
        let parsed = load_example("window.json");

        assert_eq!(parsed.spec.title, "Sample App");
        assert_eq!(parsed.spec.icon_size, Some(96));
        let window = parsed.spec.window.expect("window should exist");
        let position = window.position.expect("position should exist");
        let size = window.size.expect("size should exist");
        assert_eq!((position.x, position.y), (120, 180));
        assert_eq!((size.width, size.height), (640, 420));
    }

    #[test]
    fn parse_format_filesystem_spec_from_file_example() {
        let parsed = load_example("format_filesystem.json");

        assert_eq!(parsed.spec.title, "Sample App");
        assert_eq!(parsed.spec.format.as_deref(), Some("ULFO"));
        assert_eq!(parsed.spec.filesystem.as_deref(), Some("APFS"));
    }

    #[test]
    fn ignores_unknown_fields_like_dart() {
        let parsed = load_spec(&LoadOptions {
            source: None,
            basepath: Some(PathBuf::from(".")),
            specification: Some(json!({
                "title": "Test App",
                "badge-icon": "AppIcon.icns",
                "extra-top-level": true,
                "window": {
                    "position": { "x": 100, "y": 120, "extra": 1 },
                    "size": { "width": 600, "height": 400, "extra": 2 },
                    "unknown": "ignored"
                },
                "code-sign": {
                    "signing-identity": "Developer ID Application: Foo",
                    "legacy-option": "ignored"
                },
                "contents": [
                    { "x": 448, "y": 344, "type": "link", "path": "/Applications", "extra": "ok" },
                    { "x": 192, "y": 344, "type": "file", "path": "Test.app" }
                ]
            })),
        })
        .expect("unknown fields should be ignored");

        assert_eq!(parsed.spec.title, "Test App");
        assert_eq!(parsed.spec.contents.len(), 2);
    }

    #[test]
    fn accepts_float_coordinates_and_icon_size() {
        let parsed = load_spec(&LoadOptions {
            source: None,
            basepath: Some(PathBuf::from(".")),
            specification: Some(json!({
                "title": "Float App",
                "icon-size": 80.0,
                "window": {
                    "position": { "x": 100.0, "y": -10.5 },
                    "size": { "width": 600.0, "height": 400.4 }
                },
                "contents": [
                    { "x": 448.0, "y": 344.0, "type": "link", "path": "/Applications" },
                    { "x": 192.6, "y": 344.2, "type": "file", "path": "Float.app" }
                ]
            })),
        })
        .expect("float coordinates should deserialize");

        assert_eq!(parsed.spec.icon_size, Some(80));
        let window = parsed.spec.window.expect("window should exist");
        let pos = window.position.expect("position should exist");
        let size = window.size.expect("size should exist");
        assert_eq!((pos.x, pos.y), (100, -11));
        assert_eq!((size.width, size.height), (600, 400));
        assert_eq!(
            (parsed.spec.contents[0].x, parsed.spec.contents[0].y),
            (448, 344)
        );
        assert_eq!(
            (parsed.spec.contents[1].x, parsed.spec.contents[1].y),
            (193, 344)
        );

        // Legacy spec also accepts floats and unknown fields.
        let legacy = load_spec(&LoadOptions {
            source: None,
            basepath: Some(PathBuf::from(".")),
            specification: Some(json!({
                "title": "Legacy Float",
                "app": "Legacy.app",
                "badge-icon": "ignored.icns",
                "icons": {
                    "size": 80.0,
                    "app": [192.0, 344.0],
                    "alias": [448.0, 344.0]
                },
                "extra": [["Readme.txt", 100.0, 200.0]]
            })),
        })
        .expect("legacy float spec should deserialize");
        assert_eq!(legacy.spec.icon_size, Some(80));
        assert_eq!(
            (legacy.spec.contents[1].x, legacy.spec.contents[1].y),
            (192, 344)
        );
        assert_eq!(
            (legacy.spec.contents[2].x, legacy.spec.contents[2].y),
            (100, 200)
        );

        // Negative icon-size is rejected.
        let negative = load_spec(&LoadOptions {
            source: None,
            basepath: Some(PathBuf::from(".")),
            specification: Some(json!({
                "title": "Bad Icon Size",
                "icon-size": -1.0,
                "contents": [
                    { "x": 192, "y": 344, "type": "file", "path": "App.app" }
                ]
            })),
        });
        assert!(negative.is_err());
    }
}
