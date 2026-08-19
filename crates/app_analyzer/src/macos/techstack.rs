use crate::macho::MachOInfo;
use crate::macos::{asar, fs_stats};
use crate::{json_util, linking, plist_util, sdks};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// Describes the technology an app is built on: its runtime, the languages and
/// UI toolkits it links against, the toolchain that produced it, and the
/// libraries it depends on.
///
/// The link tables come from the main executable's Mach-O load commands, so
/// they describe what the app itself links — code reached only through an
/// embedded framework is reported by that framework's own entry instead.
pub fn collect(app_path: &Path, macho: Option<&MachOInfo>) -> Map<String, Value> {
    let frameworks_dir = app_path.join("Contents").join("Frameworks");
    let bundled_frameworks = frameworks_on_disk(&frameworks_dir);

    let mut stack = Map::new();
    let runtime = detect_runtime(app_path, &frameworks_dir, &bundled_frameworks);
    stack.insert(
        "runtime".to_string(),
        Value::String(runtime.kind.to_string()),
    );
    json_util::insert_object(&mut stack, runtime.kind, runtime.details);

    let links = macho
        .map(|macho| linking::classify(&macho.libraries))
        .unwrap_or_default();
    json_util::insert_text_array(
        &mut stack,
        "languages",
        Some(linking::languages(&links, macho)),
    );
    json_util::insert_text_array(&mut stack, "uiToolkits", Some(linking::ui_toolkits(&links)));
    if let Some(macho) = macho {
        json_util::insert_object(&mut stack, "toolchain", linking::toolchain(macho));
    }
    json_util::insert_array(
        &mut stack,
        "thirdPartySdks",
        sdks::recognize(
            links
                .embedded_frameworks
                .iter()
                .chain(links.embedded_libraries.iter())
                .chain(bundled_frameworks.iter()),
        ),
    );
    linking::insert_sections(&mut stack, links);

    stack
}

// ── Runtime ───────────────────────────────────────────────────────────────────

/// The runtime an app is built on, plus whatever that runtime reveals about
/// itself (engine revision, bundled package manifest, …).
struct Runtime {
    kind: &'static str,
    details: Map<String, Value>,
}

impl Runtime {
    fn new(kind: &'static str, details: Map<String, Value>) -> Self {
        Self { kind, details }
    }
}

fn detect_runtime(
    app_path: &Path,
    frameworks_dir: &Path,
    bundled_frameworks: &[String],
) -> Runtime {
    if let Some(flutter) = flutter_runtime(frameworks_dir, bundled_frameworks) {
        return Runtime::new("flutter", flutter);
    }
    if let Some(electron) = electron_runtime(app_path, frameworks_dir) {
        return Runtime::new("electron", electron);
    }
    if let Some(version) = bundle_version(&frameworks_dir.join("QtCore.framework")) {
        let mut qt = Map::new();
        qt.insert("version".to_string(), Value::String(version));
        return Runtime::new("qt", qt);
    }
    if let Some(java) = java_runtime(app_path) {
        return Runtime::new("java", java);
    }
    Runtime::new("native", Map::new())
}

fn flutter_runtime(
    frameworks_dir: &Path,
    bundled_frameworks: &[String],
) -> Option<Map<String, Value>> {
    let engine_framework = frameworks_dir.join("FlutterMacOS.framework");
    let assets = frameworks_dir
        .join("App.framework")
        .join("Resources")
        .join("flutter_assets");
    if !engine_framework.exists() && !assets.exists() {
        return None;
    }

    let mut flutter = Map::new();
    if let Some(info) = bundle_info_plist(&engine_framework) {
        json_util::insert_text(
            &mut flutter,
            "engineRevision",
            plist_util::text(&info, "FlutterEngine"),
        );
        json_util::insert_text(
            &mut flutter,
            "buildMode",
            plist_util::text(&info, "BuildMode"),
        );
    }
    if assets.exists() {
        let stats = fs_stats::collect(&assets, 0);
        flutter.insert(
            "assets".to_string(),
            json!({ "fileCount": stats.file_count, "sizeBytes": stats.size_bytes }),
        );
        // A JIT (debug) build ships the kernel blob next to the assets; a
        // release build compiles Dart ahead of time into App.framework.
        flutter.insert(
            "aot".to_string(),
            Value::Bool(!assets.join("kernel_blob.bin").exists()),
        );
        flutter.insert(
            "nativeAssets".to_string(),
            Value::Bool(assets.join("NativeAssetsManifest.json").exists()),
        );
    }
    json_util::insert_text_array(
        &mut flutter,
        "plugins",
        Some(flutter_plugins(bundled_frameworks)),
    );
    Some(flutter)
}

/// Each Flutter plugin ships as a framework named after its pub package, which
/// is `lowercase_with_underscores` by convention — that is what separates them
/// from the engine and from hand-added third-party frameworks.
fn flutter_plugins(bundled_frameworks: &[String]) -> Vec<String> {
    bundled_frameworks
        .iter()
        .filter(|name| {
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
        .cloned()
        .collect()
}

fn electron_runtime(app_path: &Path, frameworks_dir: &Path) -> Option<Map<String, Value>> {
    let electron_framework = frameworks_dir.join("Electron Framework.framework");
    if !electron_framework.exists() {
        return None;
    }

    let mut electron = Map::new();
    json_util::insert_text(
        &mut electron,
        "version",
        bundle_version(&electron_framework),
    );

    let resources = app_path.join("Contents").join("Resources");
    let archive = resources.join("app.asar");
    if archive.exists() {
        electron.insert(
            "asar".to_string(),
            json!({
                "sizeBytes": fs_stats::size_of(&archive),
                "unpacked": resources.join("app.asar.unpacked").exists(),
            }),
        );
        json_util::insert_object(&mut electron, "package", package_manifest(&archive));
    }
    Some(electron)
}

/// Name, version and declared dependencies from the `package.json` inside the
/// asar archive — the JavaScript half of an Electron app's dependency list.
fn package_manifest(archive: &Path) -> Map<String, Value> {
    let Some(manifest) = asar::read_package_json(archive) else {
        return Map::new();
    };

    let mut package = Map::new();
    json_util::insert_text(
        &mut package,
        "name",
        manifest
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string),
    );
    json_util::insert_text(
        &mut package,
        "version",
        manifest
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string),
    );
    if let Some(dependencies) = manifest.get("dependencies").and_then(Value::as_object)
        && !dependencies.is_empty()
    {
        package.insert("dependencyCount".to_string(), json!(dependencies.len()));
        package.insert(
            "dependencies".to_string(),
            Value::Object(dependencies.clone()),
        );
    }
    package
}

fn java_runtime(app_path: &Path) -> Option<Map<String, Value>> {
    let contents = app_path.join("Contents");
    let runtime = contents.join("runtime");
    let app_dir = contents.join("app");
    if !runtime.exists() && !app_dir.exists() {
        return None;
    }

    let mut java = Map::new();
    java.insert("bundledRuntime".to_string(), Value::Bool(runtime.exists()));
    let jars: Vec<String> = read_dir_names(&app_dir)
        .into_iter()
        .filter(|name| name.ends_with(".jar"))
        .collect();
    if !jars.is_empty() {
        java.insert("jarCount".to_string(), json!(jars.len()));
        json_util::insert_text_array(&mut java, "jars", Some(jars));
    }
    Some(java)
}

// ── Bundle helpers ────────────────────────────────────────────────────────────

fn frameworks_on_disk(frameworks_dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = read_dir_names(frameworks_dir)
        .into_iter()
        .filter_map(|name| name.strip_suffix(".framework").map(str::to_string))
        .collect();
    names.sort();
    names
}

fn read_dir_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect()
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

fn bundle_version(path: &Path) -> Option<String> {
    let info = bundle_info_plist(path)?;
    plist_util::text(&info, "CFBundleShortVersionString")
        .or_else(|| plist_util::text(&info, "CFBundleVersion"))
}
