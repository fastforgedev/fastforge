use fastforge_core::AnalyzeError;
use plist::{Dictionary, Value};
use serde_json::{Map, Value as Json};
use std::io::Cursor;
use std::path::Path;

/// Reads a property list file and returns its root dictionary.
pub fn read_dictionary(path: &Path) -> Result<Dictionary, AnalyzeError> {
    let value = Value::from_file(path)
        .map_err(|e| AnalyzeError::Parse(format!("Failed to parse {}: {}", path.display(), e)))?;
    value
        .into_dictionary()
        .ok_or_else(|| AnalyzeError::Parse(format!("{} root is not a dictionary", path.display())))
}

/// Same as [`read_dictionary`], but returns `None` instead of an error — used
/// for the many optional plists embedded in a bundle (frameworks, helpers…).
pub fn read_dictionary_opt(path: &Path) -> Option<Dictionary> {
    Value::from_file(path).ok()?.into_dictionary()
}

pub fn parse_dictionary(bytes: &[u8]) -> Option<Dictionary> {
    Value::from_reader(Cursor::new(bytes))
        .ok()?
        .into_dictionary()
}

/// Reads `key` as text. Property lists in the wild often store numbers or
/// booleans where a string is expected (a `CFBundleVersion` of `1`, say), so
/// scalars are stringified rather than rejected.
pub fn text(dict: &Dictionary, key: &str) -> Option<String> {
    match dict.get(key)? {
        Value::String(value) => Some(value.clone()),
        Value::Integer(value) => Some(value.to_string()),
        Value::Real(value) => Some(value.to_string()),
        Value::Boolean(value) => Some(value.to_string()),
        Value::Date(value) => Some(value.to_xml_format()),
        _ => None,
    }
}

pub fn require_text(dict: &Dictionary, key: &str) -> Result<String, AnalyzeError> {
    text(dict, key).ok_or_else(|| AnalyzeError::Parse(format!("Missing {} in Info.plist", key)))
}

/// Reads `key` as a boolean, tolerating the `"YES"` / `"1"` string forms.
pub fn flag(dict: &Dictionary, key: &str) -> Option<bool> {
    match dict.get(key)? {
        Value::Boolean(value) => Some(*value),
        Value::Integer(value) => Some(value.as_signed().unwrap_or(0) != 0),
        Value::String(value) => match value.to_ascii_lowercase().as_str() {
            "yes" | "true" | "1" => Some(true),
            "no" | "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

pub fn text_array(dict: &Dictionary, key: &str) -> Option<Vec<String>> {
    let array = dict.get(key)?.as_array()?;
    let values: Vec<String> = array
        .iter()
        .filter_map(|value| value.as_string().map(str::to_string))
        .collect();
    (!values.is_empty()).then_some(values)
}

pub fn dictionary_array<'a>(dict: &'a Dictionary, key: &str) -> Option<Vec<&'a Dictionary>> {
    let array = dict.get(key)?.as_array()?;
    let values: Vec<&Dictionary> = array.iter().filter_map(Value::as_dictionary).collect();
    (!values.is_empty()).then_some(values)
}

/// Converts an arbitrary property list value into JSON. Data blobs are reported
/// by length instead of being inlined, and dates use the XML (RFC 3339) form.
pub fn to_json(value: &Value) -> Json {
    match value {
        Value::String(value) => Json::String(value.clone()),
        Value::Boolean(value) => Json::Bool(*value),
        Value::Integer(value) => value
            .as_signed()
            .map(Json::from)
            .or_else(|| value.as_unsigned().map(Json::from))
            .unwrap_or(Json::Null),
        Value::Real(value) => serde_json::Number::from_f64(*value)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        Value::Date(value) => Json::String(value.to_xml_format()),
        Value::Data(value) => Json::String(format!("<{} bytes of data>", value.len())),
        Value::Uid(value) => Json::from(value.get()),
        Value::Array(values) => Json::Array(values.iter().map(to_json).collect()),
        Value::Dictionary(dict) => {
            let mut map = Map::new();
            for (key, value) in dict.iter() {
                map.insert(key.clone(), to_json(value));
            }
            Json::Object(map)
        }
        _ => Json::Null,
    }
}
