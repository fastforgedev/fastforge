use serde_json::{Map, Value};

/// Inserts `key` only when a value is actually present, so the analysis output
/// stays free of `null`s for metadata the artifact simply does not carry.
pub fn insert_text(map: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        map.insert(key.to_string(), Value::String(value));
    }
}

pub fn insert_flag(map: &mut Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::Bool(value));
    }
}

pub fn insert_text_array(map: &mut Map<String, Value>, key: &str, values: Option<Vec<String>>) {
    if let Some(values) = values.filter(|values| !values.is_empty()) {
        map.insert(
            key.to_string(),
            Value::Array(values.into_iter().map(Value::String).collect()),
        );
    }
}

/// Inserts a number when one was found.
pub fn insert_number<T: Into<Value>>(map: &mut Map<String, Value>, key: &str, value: Option<T>) {
    if let Some(value) = value {
        map.insert(key.to_string(), value.into());
    }
}

pub fn insert_value(map: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        map.insert(key.to_string(), value);
    }
}

/// Inserts an array, skipping the key when there is nothing to report.
pub fn insert_array(map: &mut Map<String, Value>, key: &str, values: Vec<Value>) {
    if !values.is_empty() {
        map.insert(key.to_string(), Value::Array(values));
    }
}

/// Inserts a nested object, skipping the key when it would be empty.
pub fn insert_object(map: &mut Map<String, Value>, key: &str, object: Map<String, Value>) {
    if !object.is_empty() {
        map.insert(key.to_string(), Value::Object(object));
    }
}
