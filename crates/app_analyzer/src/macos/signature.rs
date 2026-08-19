use crate::command;
use crate::json_util::insert_text;
use crate::{plist_util, provisioning};
use serde_json::{Map, Value, json};
use std::path::Path;

/// Which Gatekeeper policy `spctl` should evaluate the artifact against.
#[derive(Clone, Copy)]
pub enum AssessmentType {
    /// Application bundles.
    Execute,
    /// Installer-style artifacts opened by the user, such as a DMG.
    Open,
}

impl AssessmentType {
    fn as_str(self) -> &'static str {
        match self {
            AssessmentType::Execute => "exec",
            AssessmentType::Open => "open",
        }
    }
}

/// Describes the code signature of an app bundle or disk image.
///
/// Returns `None` when `codesign` itself is unavailable, so callers can leave
/// the key out instead of claiming the artifact is unsigned. Gatekeeper and
/// notarization are only probed for signed artifacts — both are meaningless
/// otherwise, and skipping them keeps unsigned local builds fast to analyze.
pub fn inspect(path: &Path, assessment: AssessmentType) -> Option<Value> {
    let path_str = path.to_string_lossy().into_owned();
    let output = command::run("codesign", &["-dv", "--verbose=4", &path_str])?;
    let report = output.stderr_text();

    if !output.success {
        let reason = report
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("codesign failed");
        return Some(json!({ "signed": false, "reason": reason }));
    }

    let fields = parse_fields(&report);
    let flags = parse_flags(&report);
    let authorities: Vec<String> = fields
        .iter()
        .filter(|(key, _)| key == "Authority")
        .map(|(_, value)| value.clone())
        .collect();
    let adhoc = flags.iter().any(|flag| flag == "adhoc")
        || first(&fields, "Signature").is_some_and(|value| value == "adhoc");

    let mut signature = Map::new();
    signature.insert("signed".to_string(), Value::Bool(true));
    signature.insert(
        "signingType".to_string(),
        Value::String(signing_type(adhoc, authorities.first()).to_string()),
    );
    signature.insert("adhoc".to_string(), Value::Bool(adhoc));
    signature.insert(
        "hardenedRuntime".to_string(),
        Value::Bool(flags.iter().any(|flag| flag == "runtime")),
    );
    insert_text(&mut signature, "identifier", first(&fields, "Identifier"));
    insert_text(
        &mut signature,
        "teamIdentifier",
        first(&fields, "TeamIdentifier").filter(|value| value != "not set"),
    );
    if !authorities.is_empty() {
        signature.insert(
            "authorities".to_string(),
            Value::Array(authorities.into_iter().map(Value::String).collect()),
        );
    }
    if !flags.is_empty() {
        signature.insert(
            "flags".to_string(),
            Value::Array(flags.into_iter().map(Value::String).collect()),
        );
    }
    insert_text(&mut signature, "format", first(&fields, "Format"));
    insert_text(&mut signature, "cdHash", first(&fields, "CDHash"));
    insert_text(
        &mut signature,
        "hashType",
        first(&fields, "Hash type")
            .and_then(|value| value.split_whitespace().next().map(str::to_string)),
    );
    insert_text(
        &mut signature,
        "timestamp",
        first(&fields, "Timestamp").or_else(|| first(&fields, "Signed Time")),
    );
    insert_text(
        &mut signature,
        "runtimeVersion",
        first(&fields, "Runtime Version"),
    );

    if let Some(entitlements) = entitlements(&path_str) {
        signature.insert("entitlements".to_string(), entitlements);
    }
    if let Some(notarization) = notarization(&path_str) {
        signature.insert("notarization".to_string(), notarization);
    }
    if let Some(gatekeeper) = gatekeeper(&path_str, assessment) {
        signature.insert("gatekeeper".to_string(), gatekeeper);
    }

    Some(Value::Object(signature))
}

fn entitlements(path: &str) -> Option<Value> {
    let output = command::run("codesign", &["-d", "--entitlements", "-", "--xml", path])?;
    if !output.success || output.stdout.is_empty() {
        return None;
    }
    let bytes = provisioning::extract_embedded_plist(&output.stdout)
        .unwrap_or_else(|| output.stdout.clone());
    let dict = plist_util::parse_dictionary(&bytes)?;
    if dict.is_empty() {
        return None;
    }
    Some(plist_util::to_json(&plist::Value::Dictionary(dict)))
}

fn notarization(path: &str) -> Option<Value> {
    let output = command::run("xcrun", &["stapler", "validate", path])?;
    let message = output
        .combined_text()
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty() && !line.starts_with("Processing:"))
        .map(str::to_string);

    let mut notarization = Map::new();
    notarization.insert("stapled".to_string(), Value::Bool(output.success));
    insert_text(&mut notarization, "message", message);
    Some(Value::Object(notarization))
}

fn gatekeeper(path: &str, assessment: AssessmentType) -> Option<Value> {
    let mut args = vec!["--assess", "--verbose=4", "--type", assessment.as_str()];
    if matches!(assessment, AssessmentType::Open) {
        // Without this, a DMG is assessed as an arbitrary quarantined document
        // rather than against the signature it actually ships with.
        args.extend_from_slice(&["--context", "context:primary-signature"]);
    }
    args.push(path);

    let output = command::run("spctl", &args)?;
    let report = output.combined_text();

    let mut gatekeeper = Map::new();
    gatekeeper.insert("accepted".to_string(), Value::Bool(output.success));
    insert_text(&mut gatekeeper, "source", find_prefixed(&report, "source="));
    insert_text(&mut gatekeeper, "origin", find_prefixed(&report, "origin="));
    Some(Value::Object(gatekeeper))
}

/// `codesign` prints `key=value` lines; a key can repeat (`Authority`).
fn parse_fields(report: &str) -> Vec<(String, String)> {
    report
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

fn first(fields: &[(String, String)], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|(field, _)| field == key)
        .map(|(_, value)| value.clone())
}

/// Extracts the flag names from the `CodeDirectory` line, e.g.
/// `flags=0x10000(runtime)` or `flags=0x10002(adhoc,runtime)`.
fn parse_flags(report: &str) -> Vec<String> {
    report
        .lines()
        .find(|line| line.starts_with("CodeDirectory "))
        .and_then(|line| {
            line.split_whitespace()
                .find_map(|token| token.strip_prefix("flags="))
        })
        .and_then(|flags| flags.split_once('(')?.1.strip_suffix(')'))
        .map(|names| {
            names
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty() && *name != "none")
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn find_prefixed(report: &str, prefix: &str) -> Option<String> {
    report
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(prefix))
        .map(str::to_string)
}

fn signing_type(adhoc: bool, authority: Option<&String>) -> &'static str {
    if adhoc {
        return "adhoc";
    }
    let Some(authority) = authority.map(String::as_str) else {
        return "unknown";
    };
    if authority.starts_with("Developer ID Application") {
        "developer-id"
    } else if authority.starts_with("Apple Development") || authority.starts_with("Mac Developer") {
        "development"
    } else if authority.starts_with("Apple Distribution")
        || authority.starts_with("3rd Party Mac Developer Application")
    {
        "app-store-distribution"
    } else if authority.starts_with("Apple Mac OS Application Signing") {
        "mac-app-store"
    } else if authority.starts_with("Software Signing")
        || authority.starts_with("Apple Code Signing")
    {
        "apple-system"
    } else {
        "other"
    }
}
