use crate::json_util;
use crate::macos::fs_stats;
use crate::plist_util;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// Describes what is embedded inside a `.app`: frameworks, helper apps, XPC
/// services, plug-ins and login items. These are what usually account for the
/// bulk of a bundle's size, and each one is separately code signed.
pub fn collect(app_path: &Path) -> Map<String, Value> {
    let contents = app_path.join("Contents");
    let mut components = Map::new();

    let mut frameworks = Vec::new();
    let mut libraries = Vec::new();
    let mut helper_apps = Vec::new();
    for path in children(&contents.join("Frameworks")) {
        match extension_of(&path).as_deref() {
            Some("framework") => frameworks.push(describe_bundle(&path)),
            Some("app") => helper_apps.push(describe_bundle(&path)),
            Some("dylib" | "so" | "node") => libraries.push(describe_file(&path)),
            _ => {}
        }
    }
    for path in children(&contents.join("Library").join("LoginItems")) {
        if extension_of(&path).as_deref() == Some("app") {
            helper_apps.push(describe_bundle(&path));
        }
    }

    let xpc_services: Vec<Value> = children(&contents.join("XPCServices"))
        .iter()
        .chain(children(&contents.join("Library").join("XPCServices")).iter())
        .filter(|path| extension_of(path).as_deref() == Some("xpc"))
        .map(|path| describe_bundle(path))
        .collect();
    let plugins: Vec<Value> = children(&contents.join("PlugIns"))
        .iter()
        .map(|path| describe_bundle(path))
        .collect();

    json_util::insert_array(&mut components, "frameworks", frameworks);
    json_util::insert_array(&mut components, "libraries", libraries);
    json_util::insert_array(&mut components, "helperApps", helper_apps);
    json_util::insert_array(&mut components, "xpcServices", xpc_services);
    json_util::insert_array(&mut components, "plugins", plugins);
    components
}

/// Describes a nested bundle (framework, helper app, XPC service, plug-in).
fn describe_bundle(path: &Path) -> Value {
    let mut item = Map::new();
    json_util::insert_text(&mut item, "name", file_name(path));
    item.insert("sizeBytes".to_string(), json!(fs_stats::size_of(path)));
    if let Some(info) = bundle_info_plist(path) {
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
    }
    Value::Object(item)
}

fn describe_file(path: &Path) -> Value {
    let mut item = Map::new();
    json_util::insert_text(&mut item, "name", file_name(path));
    item.insert("sizeBytes".to_string(), json!(fs_stats::size_of(path)));
    Value::Object(item)
}

/// Reads the `Info.plist` of a nested bundle, which lives under `Resources/`
/// for frameworks and under `Contents/` for app-style bundles.
fn bundle_info_plist(path: &Path) -> Option<plist::Dictionary> {
    let candidates = [
        path.join("Contents").join("Info.plist"),
        path.join("Resources").join("Info.plist"),
        path.join("Versions")
            .join("A")
            .join("Resources")
            .join("Info.plist"),
    ];
    candidates
        .iter()
        .find(|candidate| candidate.exists())
        .and_then(|candidate| plist_util::read_dictionary_opt(candidate))
}

fn children(dir: &Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| !is_hidden(path))
        .collect();
    paths.sort();
    paths
}

fn is_hidden(path: &Path) -> bool {
    file_name(path).is_some_and(|name| name.starts_with('.'))
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
}
