use crate::android::badging::{Badging, Identity};
use crate::android::techstack::Layout;
use crate::android::{badging, dependencies, sdk, techstack};
use crate::archive;
use crate::archive::Archive;
use crate::command;
use crate::json_util;
use fastforge_core::{AnalyzeConfig, AnalyzeError, AnalyzeResult, AppAnalyzer};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::env;
use std::path::Path;

/// Maven coordinates the Android Gradle plugin records inside every bundle.
const DEPENDENCIES_ENTRY: &str =
    "BUNDLE-METADATA/com.android.tools.build.libraries/dependencies.pb";

pub struct AndroidAabAnalyzer;

impl AppAnalyzer for AndroidAabAnalyzer {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        "android-aab"
    }

    fn is_supported_on_current_platform(&self) -> bool {
        true
    }

    fn perform_analyze(&self, config: &AnalyzeConfig) -> Result<AnalyzeResult, AnalyzeError> {
        let aab_path = Path::new(&config.path);
        if !aab_path.is_file() {
            return Err(AnalyzeError::NotFound(format!(
                "App bundle not found: {}",
                config.path
            )));
        }

        let badging = read_badging(&config.path)?;
        let mut archive = Archive::open(aab_path)?;
        let layout = Layout::aab("base");

        let mut data = Map::new();
        data.insert("platform".to_string(), Value::String("android".to_string()));
        data.insert("format".to_string(), Value::String("aab".to_string()));
        data.append(&mut badging::identity_fields(&badging.identity));
        data.append(&mut super::artifact_fields(aab_path));

        let mut abis = badging.abis.clone();
        if abis.is_empty() {
            abis = techstack::native_abis(&archive, &layout);
        }
        json_util::insert_text_array(&mut data, "abis", Some(abis));
        json_util::insert_object(&mut data, "manifest", badging.manifest);

        let mut tech_stack = techstack::collect(&mut archive, &layout);
        // A bundle ships its full dependency graph, which is richer than the
        // `META-INF` version markers an APK carries.
        if let Some(bytes) = archive.read_bytes(DEPENDENCIES_ENTRY) {
            json_util::insert_array(&mut tech_stack, "dependencies", dependencies::parse(&bytes));
        }
        json_util::insert_object(&mut data, "techStack", tech_stack);

        json_util::insert_array(&mut data, "modules", modules(&archive));
        json_util::insert_object(
            &mut data,
            "contents",
            archive::contents_summary(archive.entries(), &layout.module),
        );
        json_util::insert_object(&mut data, "signature", signature(&archive));

        log::info!("AAB analysis completed for {}", config.path);
        Ok(AnalyzeResult::new(true, Value::Object(data)))
    }
}

/// The base module plus every dynamic feature the bundle ships.
fn modules(archive: &Archive) -> Vec<Value> {
    let mut names: Vec<String> = archive
        .entries()
        .iter()
        .filter_map(|entry| {
            let (module, rest) = entry.name.split_once('/')?;
            (rest == "manifest/AndroidManifest.xml").then(|| module.to_string())
        })
        .collect();
    names.sort();
    names.dedup();

    names
        .into_iter()
        .map(|name| {
            let prefix = format!("{}/", name);
            json!({
                "name": name,
                "sizeBytes": archive.size_under(&prefix),
                "dexCount": archive.count_under(&format!("{}dex/", prefix)),
                "hasNativeLibraries": archive.contains_prefix(&format!("{}lib/", prefix)),
                "hasAssets": archive.contains_prefix(&format!("{}assets/", prefix)),
            })
        })
        .collect()
}

/// Bundles are JAR-signed when signed at all — Play re-signs the APKs it
/// generates, so an unsigned bundle is normal.
fn signature(archive: &Archive) -> Map<String, Value> {
    let jar_signed = archive.any(|name| {
        name.starts_with("META-INF/")
            && (name.ends_with(".RSA") || name.ends_with(".DSA") || name.ends_with(".EC"))
    });

    let mut signature = Map::new();
    signature.insert("jarSigned".to_string(), Value::Bool(jar_signed));
    signature
}

// ── Identity ──────────────────────────────────────────────────────────────────

/// Reads the bundle's manifest, preferring `aapt2` and falling back to
/// `bundletool` when the SDK is not available.
fn read_badging(path: &str) -> Result<Badging, AnalyzeError> {
    if let Some(aapt2) = sdk::build_tool("aapt2")
        && let Some(output) = command::run(&aapt2.to_string_lossy(), &["dump", "badging", path])
        && output.success
        && let Ok(badging) = badging::parse(&output.stdout_text())
    {
        return Ok(badging);
    }

    read_badging_with_bundletool(path)
}

fn read_badging_with_bundletool(path: &str) -> Result<Badging, AnalyzeError> {
    let bundletool = env::var("BUNDLETOOL")
        .ok()
        .filter(|value| !value.is_empty());
    let (program, mut args) = match bundletool.as_deref() {
        Some(jar) if jar.ends_with(".jar") => ("java", vec!["-jar", jar]),
        Some(binary) => (binary, Vec::new()),
        None => ("bundletool", Vec::new()),
    };
    args.extend_from_slice(&["dump", "manifest", "--bundle", path, "--module", "base"]);

    let output = command::run(program, &args).ok_or_else(|| {
        AnalyzeError::NotFound(
            "aapt2 in Android build-tools, or bundletool (set ANDROID_HOME or BUNDLETOOL)"
                .to_string(),
        )
    })?;
    if !output.success {
        return Err(AnalyzeError::CommandFailed {
            command: "bundletool".to_string(),
            stderr: output.stderr_text(),
        });
    }

    parse_manifest_xml(&output.stdout_text())
}

fn parse_manifest_xml(manifest_xml: &str) -> Result<Badging, AnalyzeError> {
    let package = attribute(manifest_xml, "package").ok_or_else(|| {
        AnalyzeError::Parse("Failed to extract package name from manifest".to_string())
    })?;
    let version_name = attribute(manifest_xml, "android:versionName").ok_or_else(|| {
        AnalyzeError::Parse("Failed to extract version name from manifest".to_string())
    })?;
    let version_code = attribute(manifest_xml, "android:versionCode")
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| {
            AnalyzeError::Parse("Failed to parse version code as integer".to_string())
        })?;
    // A label of `@ref/…` points into the resource table, which the manifest
    // dump does not resolve; the package name is the better answer then.
    let label = attribute(manifest_xml, "android:label")
        .filter(|label| !label.starts_with('@'))
        .unwrap_or_else(|| package.clone());

    let mut manifest = Map::new();
    json_util::insert_number(
        &mut manifest,
        "minSdkVersion",
        attribute(manifest_xml, "android:minSdkVersion").and_then(|v| v.parse::<i64>().ok()),
    );
    json_util::insert_number(
        &mut manifest,
        "targetSdkVersion",
        attribute(manifest_xml, "android:targetSdkVersion").and_then(|v| v.parse::<i64>().ok()),
    );
    json_util::insert_number(
        &mut manifest,
        "compileSdkVersion",
        attribute(manifest_xml, "android:compileSdkVersion").and_then(|v| v.parse::<i64>().ok()),
    );
    json_util::insert_text_array(
        &mut manifest,
        "permissions",
        Some(permissions(manifest_xml)),
    );

    Ok(Badging {
        identity: Identity {
            package,
            label,
            version_name,
            version_code,
        },
        manifest,
        abis: Vec::new(),
    })
}

fn attribute(manifest_xml: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"{}="([^"]+)""#, regex::escape(name));
    Regex::new(&pattern)
        .ok()?
        .captures(manifest_xml)?
        .get(1)
        .map(|value| value.as_str().to_string())
}

fn permissions(manifest_xml: &str) -> Vec<String> {
    let Ok(pattern) = Regex::new(r#"<uses-permission[^>]*android:name="([^"]+)""#) else {
        return Vec::new();
    };
    let mut names: Vec<String> = pattern
        .captures_iter(manifest_xml)
        .filter_map(|capture| capture.get(1))
        .map(|value| value.as_str().to_string())
        .collect();
    names.dedup();
    names
}
