use crate::android::sdk;
use crate::command;
use crate::json_util;
use serde_json::{Map, Value};

/// Reports how an APK is signed, using `apksigner` from the Android SDK.
///
/// Returns `None` when `apksigner` is unavailable, so the key can be left out
/// rather than implying the APK is unsigned.
pub fn inspect(apk_path: &str) -> Option<Value> {
    let apksigner = sdk::build_tool("apksigner")?;
    let output = command::run(
        &apksigner.to_string_lossy(),
        &["verify", "-v", "--print-certs", apk_path],
    )?;
    let report = output.combined_text();

    let mut signature = Map::new();
    signature.insert("verified".to_string(), Value::Bool(output.success));
    if !output.success {
        json_util::insert_text(
            &mut signature,
            "reason",
            report
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string),
        );
        return Some(Value::Object(signature));
    }

    json_util::insert_text_array(&mut signature, "schemes", Some(schemes(&report)));
    json_util::insert_array(&mut signature, "signers", signers(&report));
    Some(Value::Object(signature))
}

/// The signing schemes that verified, from lines such as
/// `Verified using v2 scheme (APK Signature Scheme v2): true`.
fn schemes(report: &str) -> Vec<String> {
    report
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Verified using "))
        .filter_map(|line| {
            let (scheme, verified) = line.split_once(' ')?;
            let verified = verified.rsplit(':').next()?.trim();
            (verified == "true").then(|| scheme.to_string())
        })
        .collect()
}

/// Certificate fields worth reporting, keyed by how `apksigner` labels them.
const SIGNER_FIELDS: &[(&str, &str)] = &[
    ("certificate DN:", "subject"),
    ("certificate SHA-256 digest:", "sha256"),
    ("key algorithm:", "keyAlgorithm"),
    ("key size (bits):", "keySizeBits"),
];

/// Collects the certificates `apksigner` printed.
///
/// Build-tools revisions disagree on how they label a signer — older ones write
/// `Signer #1 certificate DN: …`, newer ones `V2 Signer: certificate DN: …` —
/// so lines are grouped by whatever label precedes the field instead.
fn signers(report: &str) -> Vec<Value> {
    let mut signers: Vec<(String, Map<String, Value>)> = Vec::new();

    for line in report.lines().map(str::trim) {
        let Some((label, key, value)) = signer_field(line) else {
            continue;
        };
        let signer = match signers.iter_mut().find(|(name, _)| *name == label) {
            Some((_, signer)) => signer,
            None => {
                signers.push((label, Map::new()));
                &mut signers.last_mut().expect("just pushed").1
            }
        };
        if key == "keySizeBits" {
            json_util::insert_number(signer, key, value.parse::<u32>().ok());
        } else {
            json_util::insert_text(signer, key, Some(value));
        }
    }

    // The same certificate is listed once per signing scheme it signed with.
    let mut seen: Vec<String> = Vec::new();
    signers
        .into_iter()
        .filter(|(_, signer)| {
            let Some(digest) = signer.get("sha256").and_then(Value::as_str) else {
                return true;
            };
            let first_time = !seen.iter().any(|value| value == digest);
            seen.push(digest.to_string());
            first_time
        })
        .map(|(_, signer)| Value::Object(signer))
        .collect()
}

fn signer_field(line: &str) -> Option<(String, &'static str, String)> {
    if !line.contains("Signer") {
        return None;
    }
    SIGNER_FIELDS.iter().find_map(|(marker, key)| {
        let start = line.find(marker)?;
        let label = line[..start].trim().trim_end_matches(':').trim();
        Some((
            label.to_string(),
            *key,
            line[start + marker.len()..].trim().to_string(),
        ))
    })
}
