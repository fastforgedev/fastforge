use crate::json_util;
use fastforge_core::AnalyzeError;
use serde_json::{Map, Value, json};

/// The identity every Android analyzer must report.
pub struct Identity {
    pub package: String,
    pub label: String,
    pub version_name: String,
    pub version_code: i64,
}

/// Everything `aapt2 dump badging` says about a package.
pub struct Badging {
    pub identity: Identity,
    pub manifest: Map<String, Value>,
    pub abis: Vec<String>,
}

/// Parses `aapt2 dump badging` output.
///
/// The format is one record per line, with single-quoted values — either
/// `key: name='value' other='value'` or `key:'value' 'value'`.
pub fn parse(output: &str) -> Result<Badging, AnalyzeError> {
    let package_line = output
        .lines()
        .find_map(|line| line.strip_prefix("package:"))
        .ok_or_else(|| {
            AnalyzeError::Parse("Failed to extract package line from aapt output".to_string())
        })?;
    let package_fields = attributes(package_line);

    let package = field(&package_fields, "name").ok_or_else(|| {
        AnalyzeError::Parse("Failed to extract package name from aapt output".to_string())
    })?;
    let version_name = field(&package_fields, "versionName").ok_or_else(|| {
        AnalyzeError::Parse("Failed to extract version name from aapt output".to_string())
    })?;
    let version_code = field(&package_fields, "versionCode")
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| {
            AnalyzeError::Parse("Failed to parse version code as integer".to_string())
        })?;
    let label = value_of(output, "application-label").unwrap_or_else(|| package.clone());

    let mut manifest = Map::new();
    json_util::insert_number(
        &mut manifest,
        "minSdkVersion",
        number_value(output, "minSdkVersion").or_else(|| number_value(output, "sdkVersion")),
    );
    json_util::insert_number(
        &mut manifest,
        "targetSdkVersion",
        number_value(output, "targetSdkVersion"),
    );
    json_util::insert_number(
        &mut manifest,
        "compileSdkVersion",
        field(&package_fields, "compileSdkVersion").and_then(|value| value.parse::<i64>().ok()),
    );
    json_util::insert_text(
        &mut manifest,
        "platformBuildVersion",
        field(&package_fields, "platformBuildVersionName"),
    );
    json_util::insert_text(
        &mut manifest,
        "launchableActivity",
        output
            .lines()
            .find_map(|line| line.trim().strip_prefix("launchable-activity:"))
            .and_then(|line| field(&attributes(line), "name")),
    );
    json_util::insert_text_array(
        &mut manifest,
        "permissions",
        Some(named_records(output, "uses-permission:")),
    );
    json_util::insert_text_array(
        &mut manifest,
        "features",
        Some(named_records(output, "uses-feature:")),
    );
    json_util::insert_text_array(
        &mut manifest,
        "libraries",
        Some(quoted_records(
            output,
            &["uses-library:", "uses-library-not-required:"],
        )),
    );
    json_util::insert_text_array(
        &mut manifest,
        "locales",
        Some(
            list_value(output, "locales")
                .into_iter()
                // aapt2 reports the default (unqualified) resources as `--_--`.
                .filter(|locale| locale != "--_--")
                .collect(),
        ),
    );
    json_util::insert_text_array(
        &mut manifest,
        "densities",
        Some(list_value(output, "densities")),
    );
    json_util::insert_text_array(
        &mut manifest,
        "supportedScreens",
        Some(list_value(output, "supports-screens")),
    );

    let mut abis = list_value(output, "native-code");
    abis.extend(list_value(output, "alt-native-code"));
    abis.sort();
    abis.dedup();

    Ok(Badging {
        identity: Identity {
            package,
            label,
            version_name,
            version_code,
        },
        manifest,
        abis,
    })
}

/// Collects `key='value'` pairs from one record.
fn attributes(line: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut rest = line;
    while let Some(equals) = rest.find("='") {
        let key = rest[..equals]
            .rsplit(|c: char| c.is_whitespace())
            .next()
            .unwrap_or_default()
            .to_string();
        rest = &rest[equals + 2..];
        let Some(end) = rest.find('\'') else {
            break;
        };
        pairs.push((key, rest[..end].to_string()));
        rest = &rest[end + 1..];
    }
    pairs
}

fn field(pairs: &[(String, String)], key: &str) -> Option<String> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

/// Reads a `key:'value'` record, e.g. `application-label:'Example'`.
fn value_of(output: &str, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .and_then(|rest| quoted(rest).into_iter().next())
}

fn number_value(output: &str, key: &str) -> Option<i64> {
    value_of(output, key)?.parse::<i64>().ok()
}

/// Reads a `key:'a' 'b' 'c'` record, e.g. `native-code: 'arm64-v8a'`.
fn list_value(output: &str, key: &str) -> Vec<String> {
    let prefix = format!("{}:", key);
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .map(quoted)
        .unwrap_or_default()
}

/// Collects the `name='…'` of every record starting with `prefix`, keeping the
/// order aapt2 printed them in.
fn named_records(output: &str, prefix: &str) -> Vec<String> {
    let mut names: Vec<String> = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix(prefix))
        .filter_map(|line| field(&attributes(line), "name"))
        .collect();
    names.dedup();
    names
}

fn quoted_records(output: &str, prefixes: &[&str]) -> Vec<String> {
    let mut values: Vec<String> = output
        .lines()
        .map(str::trim)
        .filter_map(|line| prefixes.iter().find_map(|prefix| line.strip_prefix(prefix)))
        .flat_map(quoted)
        .collect();
    values.sort();
    values.dedup();
    values
}

/// Every single-quoted value in a record.
fn quoted(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('\'') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('\'') else {
            break;
        };
        values.push(rest[..end].to_string());
        rest = &rest[end + 1..];
    }
    values
}

/// Identity as the analyzers report it at the top level of the payload.
pub fn identity_fields(identity: &Identity) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert(
        "identifier".to_string(),
        Value::String(identity.package.clone()),
    );
    fields.insert("name".to_string(), Value::String(identity.label.clone()));
    fields.insert(
        "version".to_string(),
        Value::String(identity.version_name.clone()),
    );
    fields.insert("buildNumber".to_string(), json!(identity.version_code));
    fields
}
