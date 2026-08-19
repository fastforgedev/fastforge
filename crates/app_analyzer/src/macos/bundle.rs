use crate::json_util;
use crate::macos::signature::AssessmentType;
use crate::macos::{components, fs_stats, signature, techstack};
use crate::{macho, plist_util, provisioning};
use fastforge_core::AnalyzeError;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// How many of the biggest files inside the bundle to report.
const LARGEST_FILE_LIMIT: usize = 10;

/// Fully inspects a macOS `.app` bundle.
///
/// The returned map is the analysis payload itself for a `.app` artifact, and
/// the `app` section of the payload when the bundle was found inside a DMG.
pub fn inspect(app_path: &Path) -> Result<Map<String, Value>, AnalyzeError> {
    let contents = app_path.join("Contents");
    let info_plist = contents.join("Info.plist");
    if !info_plist.exists() {
        return Err(AnalyzeError::NotFound(format!(
            "Info.plist not found at: {}",
            info_plist.display()
        )));
    }
    let info = plist_util::read_dictionary(&info_plist)?;

    let identifier = plist_util::require_text(&info, "CFBundleIdentifier")?;
    let display_name = plist_util::text(&info, "CFBundleDisplayName");
    let bundle_name = plist_util::text(&info, "CFBundleName");
    let name = display_name
        .clone()
        .or_else(|| bundle_name.clone())
        .ok_or_else(|| {
            AnalyzeError::Parse(
                "Missing CFBundleDisplayName/CFBundleName in Info.plist".to_string(),
            )
        })?;
    let version = plist_util::require_text(&info, "CFBundleShortVersionString")?;
    let build_number = plist_util::require_text(&info, "CFBundleVersion")?;
    let executable = plist_util::text(&info, "CFBundleExecutable");

    let stats = fs_stats::collect(app_path, LARGEST_FILE_LIMIT);
    let executable_path = executable
        .as_ref()
        .map(|name| contents.join("MacOS").join(name))
        .filter(|path| path.exists());
    // The main executable answers two questions at once: which architectures
    // the app ships, and which libraries it is built on.
    let mach_o = executable_path.as_deref().and_then(macho::inspect);
    let architectures = match mach_o.as_ref() {
        Some(mach_o) if !mach_o.architectures.is_empty() => mach_o.architectures.clone(),
        _ => executable_path
            .as_deref()
            .map(macho::architectures)
            .unwrap_or_default(),
    };

    let mut app = Map::new();
    app.insert("platform".to_string(), Value::String("macos".to_string()));
    app.insert("format".to_string(), Value::String("app".to_string()));
    app.insert("identifier".to_string(), Value::String(identifier));
    app.insert("name".to_string(), Value::String(name));
    app.insert("version".to_string(), Value::String(version));
    app.insert("buildNumber".to_string(), Value::String(build_number));
    json_util::insert_text(&mut app, "displayName", display_name);
    json_util::insert_text(&mut app, "bundleName", bundle_name);
    json_util::insert_text(
        &mut app,
        "path",
        Some(
            fs::canonicalize(app_path)
                .unwrap_or_else(|_| app_path.to_path_buf())
                .to_string_lossy()
                .into_owned(),
        ),
    );
    json_util::insert_text(
        &mut app,
        "fileName",
        app_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
    );

    json_util::insert_text(&mut app, "executable", executable);
    json_util::insert_text(
        &mut app,
        "bundleType",
        plist_util::text(&info, "CFBundlePackageType"),
    );
    json_util::insert_text(
        &mut app,
        "minOSVersion",
        plist_util::text(&info, "LSMinimumSystemVersion"),
    );
    app.insert(
        "architectures".to_string(),
        Value::Array(
            architectures
                .iter()
                .map(|arch| Value::String(arch.clone()))
                .collect(),
        ),
    );
    app.insert(
        "universal".to_string(),
        Value::Bool(architectures.len() > 1),
    );
    if let Some(path) = executable_path.as_deref() {
        app.insert(
            "executableSizeBytes".to_string(),
            json!(fs_stats::size_of(path)),
        );
    }

    app.insert("sizeBytes".to_string(), json!(stats.size_bytes));
    app.insert("fileCount".to_string(), json!(stats.file_count));

    json_util::insert_text(
        &mut app,
        "category",
        plist_util::text(&info, "LSApplicationCategoryType"),
    );
    json_util::insert_text(
        &mut app,
        "copyright",
        plist_util::text(&info, "NSHumanReadableCopyright"),
    );
    json_util::insert_text(
        &mut app,
        "iconFile",
        plist_util::text(&info, "CFBundleIconFile")
            .or_else(|| plist_util::text(&info, "CFBundleIconName")),
    );
    json_util::insert_text(
        &mut app,
        "developmentRegion",
        plist_util::text(&info, "CFBundleDevelopmentRegion"),
    );
    json_util::insert_text_array(
        &mut app,
        "supportedPlatforms",
        plist_util::text_array(&info, "CFBundleSupportedPlatforms"),
    );
    json_util::insert_text_array(&mut app, "localizations", localizations(&contents, &info));
    json_util::insert_flag(&mut app, "agentApp", plist_util::flag(&info, "LSUIElement"));
    json_util::insert_flag(
        &mut app,
        "highResolutionCapable",
        plist_util::flag(&info, "NSHighResolutionCapable"),
    );
    json_util::insert_flag(
        &mut app,
        "encryptionExempt",
        plist_util::flag(&info, "ITSAppUsesNonExemptEncryption").map(|uses| !uses),
    );

    json_util::insert_object(&mut app, "buildInfo", build_info(&info));
    json_util::insert_object(
        &mut app,
        "techStack",
        techstack::collect(app_path, mach_o.as_ref()),
    );
    json_util::insert_object(&mut app, "components", components::collect(app_path));
    json_util::insert_object(&mut app, "sizeBreakdown", size_breakdown(&contents));
    app.insert(
        "largestFiles".to_string(),
        fs_stats::largest_files_json(&stats),
    );

    json_util::insert_array(&mut app, "urlSchemes", url_schemes(&info));
    json_util::insert_array(&mut app, "documentTypes", document_types(&info));
    json_util::insert_object(
        &mut app,
        "privacyUsageDescriptions",
        privacy_usage_descriptions(&info),
    );

    let code_signature = signature::inspect(app_path, AssessmentType::Execute);
    json_util::insert_flag(&mut app, "sandboxed", sandboxed(code_signature.as_ref()));
    json_util::insert_value(&mut app, "codeSignature", code_signature);
    json_util::insert_value(
        &mut app,
        "provisioningProfile",
        provisioning::read(&contents.join("embedded.provisionprofile")),
    );

    Ok(app)
}

/// Short description of a bundle, used to list the other apps a DMG ships.
pub fn summary(app_path: &Path) -> Option<Value> {
    let info = plist_util::read_dictionary_opt(&app_path.join("Contents").join("Info.plist"))?;

    let mut item = Map::new();
    json_util::insert_text(
        &mut item,
        "name",
        plist_util::text(&info, "CFBundleDisplayName")
            .or_else(|| plist_util::text(&info, "CFBundleName")),
    );
    json_util::insert_text(
        &mut item,
        "identifier",
        plist_util::text(&info, "CFBundleIdentifier"),
    );
    json_util::insert_text(
        &mut item,
        "version",
        plist_util::text(&info, "CFBundleShortVersionString"),
    );
    json_util::insert_text(
        &mut item,
        "buildNumber",
        plist_util::text(&info, "CFBundleVersion"),
    );
    item.insert("sizeBytes".to_string(), json!(fs_stats::size_of(app_path)));
    Some(Value::Object(item))
}

/// Verifies that `path` really is a `.app` bundle directory.
pub fn validate(path: &Path) -> Result<(), AnalyzeError> {
    if !path.exists() {
        return Err(AnalyzeError::NotFound(format!(
            "App bundle not found: {}",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(AnalyzeError::Parse(format!(
            "Expected a .app bundle directory, but path is not a directory: {}",
            path.display()
        )));
    }
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if extension != "app" {
        return Err(AnalyzeError::Parse(format!(
            "Expected a .app bundle, got '.{}' extension",
            extension
        )));
    }
    Ok(())
}

/// Toolchain the bundle was produced with, from the `DT*` keys Xcode writes.
fn build_info(info: &plist::Dictionary) -> Map<String, Value> {
    let mut build = Map::new();
    json_util::insert_text(&mut build, "sdk", plist_util::text(info, "DTSDKName"));
    json_util::insert_text(&mut build, "sdkBuild", plist_util::text(info, "DTSDKBuild"));
    json_util::insert_text(
        &mut build,
        "platformName",
        plist_util::text(info, "DTPlatformName"),
    );
    json_util::insert_text(
        &mut build,
        "platformVersion",
        plist_util::text(info, "DTPlatformVersion"),
    );
    json_util::insert_text(
        &mut build,
        "platformBuild",
        plist_util::text(info, "DTPlatformBuild"),
    );
    json_util::insert_text(&mut build, "xcode", plist_util::text(info, "DTXcode"));
    json_util::insert_text(
        &mut build,
        "xcodeBuild",
        plist_util::text(info, "DTXcodeBuild"),
    );
    json_util::insert_text(&mut build, "compiler", plist_util::text(info, "DTCompiler"));
    json_util::insert_text(
        &mut build,
        "machineOSBuild",
        plist_util::text(info, "BuildMachineOSBuild"),
    );
    build
}

/// Size of each top-level directory under `Contents`, largest first — the
/// quickest way to see what is making a bundle big.
fn size_breakdown(contents: &Path) -> Map<String, Value> {
    let Ok(entries) = fs::read_dir(contents) else {
        return Map::new();
    };

    let mut sizes: Vec<(String, u64)> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (name, fs_stats::collect(&entry.path(), 0).size_bytes)
        })
        .collect();
    sizes.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    sizes
        .into_iter()
        .map(|(name, size)| (name, json!(size)))
        .collect()
}

/// Languages the bundle ships, from its `.lproj` directories.
fn localizations(contents: &Path, info: &plist::Dictionary) -> Option<Vec<String>> {
    let mut names: Vec<String> = match fs::read_dir(contents.join("Resources")) {
        Ok(entries) => entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("lproj") {
                    return None;
                }
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    if let Some(declared) = plist_util::text_array(info, "CFBundleLocalizations") {
        names.extend(declared);
    }
    names.sort();
    names.dedup();
    (!names.is_empty()).then_some(names)
}

fn url_schemes(info: &plist::Dictionary) -> Vec<Value> {
    let Some(types) = plist_util::dictionary_array(info, "CFBundleURLTypes") else {
        return Vec::new();
    };

    types
        .into_iter()
        .map(|entry| {
            let mut item = Map::new();
            json_util::insert_text(
                &mut item,
                "name",
                plist_util::text(entry, "CFBundleURLName"),
            );
            json_util::insert_text(
                &mut item,
                "role",
                plist_util::text(entry, "CFBundleTypeRole"),
            );
            json_util::insert_text_array(
                &mut item,
                "schemes",
                plist_util::text_array(entry, "CFBundleURLSchemes"),
            );
            Value::Object(item)
        })
        .collect()
}

fn document_types(info: &plist::Dictionary) -> Vec<Value> {
    let Some(types) = plist_util::dictionary_array(info, "CFBundleDocumentTypes") else {
        return Vec::new();
    };

    types
        .into_iter()
        .map(|entry| {
            let mut item = Map::new();
            json_util::insert_text(
                &mut item,
                "name",
                plist_util::text(entry, "CFBundleTypeName"),
            );
            json_util::insert_text(
                &mut item,
                "role",
                plist_util::text(entry, "CFBundleTypeRole"),
            );
            json_util::insert_text_array(
                &mut item,
                "extensions",
                plist_util::text_array(entry, "CFBundleTypeExtensions"),
            );
            json_util::insert_text_array(
                &mut item,
                "contentTypes",
                plist_util::text_array(entry, "LSItemContentTypes"),
            );
            Value::Object(item)
        })
        .collect()
}

/// The `NS*UsageDescription` prompts the app can show — effectively the list of
/// protected resources it asks for.
fn privacy_usage_descriptions(info: &plist::Dictionary) -> Map<String, Value> {
    info.iter()
        .filter(|(key, _)| key.ends_with("UsageDescription"))
        .filter_map(|(key, value)| {
            Some((key.clone(), Value::String(value.as_string()?.to_string())))
        })
        .collect()
}

/// Whether the app runs in the App Sandbox. Only reported when entitlements
/// could be read at all — an app that declares none is not sandboxed, but an
/// unsigned one tells us nothing either way.
fn sandboxed(code_signature: Option<&Value>) -> Option<bool> {
    let entitlements = code_signature?.get("entitlements")?;
    Some(
        entitlements
            .get("com.apple.security.app-sandbox")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    )
}
