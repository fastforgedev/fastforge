use crate::{json_util, plist_util};
use serde_json::{Map, Value, json};
use std::path::Path;
use std::time::SystemTime;

/// Parses a provisioning profile — `embedded.provisionprofile` in a macOS
/// bundle, `embedded.mobileprovision` in an iOS app. Both are the same
/// CMS-signed property list.
pub fn read(path: &Path) -> Option<Value> {
    parse(&std::fs::read(path).ok()?)
}

pub fn parse(bytes: &[u8]) -> Option<Value> {
    let dict = plist_util::parse_dictionary(&extract_embedded_plist(bytes)?)?;

    let mut profile = Map::new();
    json_util::insert_text(&mut profile, "name", plist_util::text(&dict, "Name"));
    json_util::insert_text(
        &mut profile,
        "teamName",
        plist_util::text(&dict, "TeamName"),
    );
    json_util::insert_text(
        &mut profile,
        "teamIdentifier",
        plist_util::text_array(&dict, "TeamIdentifier").and_then(|values| values.first().cloned()),
    );
    json_util::insert_text(
        &mut profile,
        "appIdName",
        plist_util::text(&dict, "AppIDName"),
    );
    json_util::insert_text(&mut profile, "uuid", plist_util::text(&dict, "UUID"));
    json_util::insert_text(
        &mut profile,
        "distributionType",
        Some(distribution_type(&dict).to_string()),
    );
    json_util::insert_text(
        &mut profile,
        "creationDate",
        plist_util::text(&dict, "CreationDate"),
    );
    json_util::insert_text(
        &mut profile,
        "expirationDate",
        plist_util::text(&dict, "ExpirationDate"),
    );
    if let Some(expires_at) = dict.get("ExpirationDate").and_then(|value| value.as_date()) {
        let expires_at: SystemTime = expires_at.into();
        profile.insert(
            "expired".to_string(),
            Value::Bool(expires_at <= SystemTime::now()),
        );
    }
    json_util::insert_text_array(
        &mut profile,
        "platforms",
        plist_util::text_array(&dict, "Platform"),
    );
    if let Some(devices) = dict
        .get("ProvisionedDevices")
        .and_then(|value| value.as_array())
    {
        profile.insert("provisionedDeviceCount".to_string(), json!(devices.len()));
    }
    json_util::insert_flag(
        &mut profile,
        "xcodeManaged",
        plist_util::flag(&dict, "IsXcodeManaged"),
    );
    if let Some(entitlements) = dict.get("Entitlements") {
        profile.insert(
            "entitlements".to_string(),
            plist_util::to_json(entitlements),
        );
    }

    Some(Value::Object(profile))
}

/// How the profile allows the app to be distributed.
///
/// Apple encodes this indirectly: a profile listing devices is for testing —
/// development when debugging is allowed, ad-hoc when it is not — while one
/// without devices is either enterprise (any device) or App Store.
fn distribution_type(profile: &plist::Dictionary) -> &'static str {
    let entitlements = profile
        .get("Entitlements")
        .and_then(|value| value.as_dictionary());
    let debuggable = entitlements
        .and_then(|entitlements| plist_util::flag(entitlements, "get-task-allow"))
        .unwrap_or(false);

    let macos = plist_util::text_array(profile, "Platform")
        .is_some_and(|platforms| platforms.iter().any(|platform| platform == "OSX"));

    if profile.contains_key("ProvisionedDevices") {
        if debuggable { "development" } else { "ad-hoc" }
    } else if plist_util::flag(profile, "ProvisionsAllDevices").unwrap_or(false) {
        // The same flag means "any Mac" for a Developer ID profile and "any
        // enrolled device" for an iOS in-house one.
        if macos { "developer-id" } else { "enterprise" }
    } else {
        "app-store"
    }
}

/// Pulls the plist out of a container that wraps it — a CMS-signed profile, or
/// an entitlement blob with its 8-byte magic header.
pub fn extract_embedded_plist(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.starts_with(b"bplist00") {
        return Some(bytes.to_vec());
    }
    let start = find_subslice(bytes, b"<?xml")?;
    let end = find_subslice(&bytes[start..], b"</plist>").map(|offset| start + offset + 8)?;
    Some(bytes[start..end].to_vec())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
