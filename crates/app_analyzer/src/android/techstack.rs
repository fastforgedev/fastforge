use crate::archive::Archive;
use crate::json_util;
use serde_json::{Map, Value, json};

/// Where to look for the different kinds of content inside an Android package.
///
/// An APK keeps everything at the root. An AAB splits it up: code and assets
/// live under a module directory, plain Java resources under `<module>/root/`,
/// and build metadata under `BUNDLE-METADATA/`.
pub struct Layout {
    /// Prefix of the module payload, e.g. `""` or `"base/"`.
    pub module: String,
    /// Prefix of the Java resources, e.g. `""` or `"base/root/"`.
    pub root: String,
    /// Prefix of the Android Gradle plugin metadata.
    pub metadata: String,
}

impl Layout {
    pub fn apk() -> Self {
        Self {
            module: String::new(),
            root: String::new(),
            metadata: "META-INF/com/android/build/gradle/".to_string(),
        }
    }

    pub fn aab(module: &str) -> Self {
        Self {
            module: format!("{}/", module),
            root: format!("{}/root/", module),
            metadata: "BUNDLE-METADATA/com.android.tools.build.gradle/".to_string(),
        }
    }
}

/// Describes what an Android package is built with: its runtime, the languages
/// and UI toolkit in use, the build tools that produced it, and the libraries
/// it ships.
pub fn collect(archive: &mut Archive, layout: &Layout) -> Map<String, Value> {
    let mut stack = Map::new();

    let runtime = detect_runtime(archive, layout);
    stack.insert(
        "runtime".to_string(),
        Value::String(runtime.kind.to_string()),
    );
    json_util::insert_object(&mut stack, runtime.kind, runtime.details);

    let libraries = libraries(archive, layout);
    json_util::insert_text_array(
        &mut stack,
        "languages",
        Some(languages(archive, layout, runtime.kind)),
    );
    json_util::insert_text_array(&mut stack, "uiToolkits", Some(ui_toolkits(&libraries)));
    json_util::insert_object(&mut stack, "buildTools", build_tools(archive, layout));
    json_util::insert_text_array(
        &mut stack,
        "nativeLibraries",
        Some(native_libraries(archive, layout)),
    );
    json_util::insert_array(
        &mut stack,
        "libraries",
        libraries
            .iter()
            .map(|(name, version)| json!({ "name": name, "version": version }))
            .collect(),
    );

    stack
}

/// ABIs the package actually ships native code for, from its `lib/` layout.
pub fn native_abis(archive: &Archive, layout: &Layout) -> Vec<String> {
    let prefix = format!("{}lib/", layout.module);
    let mut abis: Vec<String> = archive
        .names_with_prefix(&prefix)
        .into_iter()
        .filter_map(|name| name[prefix.len()..].split('/').next())
        .map(str::to_string)
        .collect();
    abis.sort();
    abis.dedup();
    abis
}

// ── Runtime ───────────────────────────────────────────────────────────────────

struct Runtime {
    kind: &'static str,
    details: Map<String, Value>,
}

impl Runtime {
    fn new(kind: &'static str, details: Map<String, Value>) -> Self {
        Self { kind, details }
    }
}

fn detect_runtime(archive: &Archive, layout: &Layout) -> Runtime {
    let module = layout.module.as_str();

    if let Some(flutter) = flutter_runtime(archive, layout) {
        return Runtime::new("flutter", flutter);
    }
    if archive.contains(&format!("{}assets/index.android.bundle", module))
        || has_native_library(archive, layout, "libreactnativejni.so")
        || has_native_library(archive, layout, "libreactnative.so")
    {
        let mut react_native = Map::new();
        react_native.insert(
            "jsEngine".to_string(),
            Value::String(
                if has_native_library(archive, layout, "libhermes.so")
                    || has_native_library(archive, layout, "libhermesinstancejni.so")
                {
                    "hermes".to_string()
                } else {
                    "jsc".to_string()
                },
            ),
        );
        react_native.insert(
            "bundled".to_string(),
            Value::Bool(archive.contains(&format!("{}assets/index.android.bundle", module))),
        );
        return Runtime::new("react-native", react_native);
    }
    if has_native_library(archive, layout, "libunity.so")
        || archive.contains_prefix(&format!("{}assets/bin/Data/", module))
    {
        return Runtime::new("unity", Map::new());
    }
    if has_native_library(archive, layout, "libmonodroid.so")
        || archive.contains_prefix(&format!("{}assemblies/", module))
    {
        let mut dotnet = Map::new();
        let assemblies = archive.count_under(&format!("{}assemblies/", module));
        if assemblies > 0 {
            dotnet.insert("assemblyCount".to_string(), json!(assemblies));
        }
        return Runtime::new("dotnet", dotnet);
    }
    if archive.contains(&format!("{}assets/www/index.html", module)) {
        return Runtime::new("cordova", Map::new());
    }

    Runtime::new("native", Map::new())
}

fn flutter_runtime(archive: &Archive, layout: &Layout) -> Option<Map<String, Value>> {
    let assets = format!("{}assets/flutter_assets/", layout.module);
    if !archive.contains_prefix(&assets) && !has_native_library(archive, layout, "libflutter.so") {
        return None;
    }

    let mut flutter = Map::new();
    flutter.insert(
        "assets".to_string(),
        json!({
            "fileCount": archive.count_under(&assets),
            "sizeBytes": archive.size_under(&assets),
        }),
    );
    // Release builds compile Dart ahead of time into `libapp.so`; debug builds
    // ship the kernel blob next to the assets instead.
    flutter.insert(
        "aot".to_string(),
        Value::Bool(has_native_library(archive, layout, "libapp.so")),
    );
    flutter.insert(
        "nativeAssets".to_string(),
        Value::Bool(archive.contains(&format!("{}NativeAssetsManifest.json", assets))),
    );
    Some(flutter)
}

fn has_native_library(archive: &Archive, layout: &Layout, file_name: &str) -> bool {
    let prefix = format!("{}lib/", layout.module);
    let suffix = format!("/{}", file_name);
    archive.any(|name| name.starts_with(&prefix) && name.ends_with(&suffix))
}

// ── Languages, toolkits and tools ─────────────────────────────────────────────

fn languages(archive: &Archive, layout: &Layout, runtime: &str) -> Vec<String> {
    let mut languages = Vec::new();

    let uses_kotlin = archive.contains_prefix(&format!("{}kotlin/", layout.root))
        || archive.contains(&format!("{}kotlin-tooling-metadata.json", layout.root))
        || archive.any(|name| name.ends_with(".kotlin_module"));
    if uses_kotlin {
        languages.push("Kotlin".to_string());
    } else if archive.any(|name| name.ends_with(".dex")) {
        // Every Android package carries JVM bytecode; without a Kotlin marker
        // the sources are most likely Java.
        languages.push("Java".to_string());
    }

    match runtime {
        "flutter" => languages.push("Dart".to_string()),
        "react-native" | "cordova" => languages.push("JavaScript".to_string()),
        "dotnet" => languages.push("C#".to_string()),
        _ => {}
    }
    if archive.contains_prefix(&format!("{}lib/", layout.module)) {
        languages.push("C/C++".to_string());
    }
    languages
}

fn ui_toolkits(libraries: &[(String, String)]) -> Vec<String> {
    let mut toolkits = Vec::new();
    if libraries
        .iter()
        .any(|(name, _)| name.starts_with("androidx.compose"))
    {
        toolkits.push("Jetpack Compose".to_string());
    }
    if libraries
        .iter()
        .any(|(name, _)| name.starts_with("androidx.appcompat"))
    {
        toolkits.push("AppCompat".to_string());
    }
    toolkits
}

/// Versions of the tools that produced the package, from the metadata Gradle
/// and the Kotlin plugin leave behind.
fn build_tools(archive: &mut Archive, layout: &Layout) -> Map<String, Value> {
    let mut tools = Map::new();

    if let Some(properties) =
        archive.read_text(&format!("{}app-metadata.properties", layout.metadata))
    {
        json_util::insert_text(
            &mut tools,
            "androidGradlePlugin",
            property(&properties, "androidGradlePluginVersion"),
        );
    }

    if let Some(metadata) = archive
        .read_text(&format!("{}kotlin-tooling-metadata.json", layout.root))
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    {
        json_util::insert_text(
            &mut tools,
            "gradle",
            text_at(&metadata, &["buildSystemVersion"]),
        );
        json_util::insert_text(
            &mut tools,
            "kotlin",
            text_at(&metadata, &["buildPluginVersion"]),
        );
        json_util::insert_text(
            &mut tools,
            "javaTarget",
            metadata
                .get("projectTargets")
                .and_then(Value::as_array)
                .and_then(|targets| targets.first())
                .and_then(|target| text_at(target, &["extras", "android", "targetCompatibility"])),
        );
    }

    tools
}

/// Maven coordinates and versions of the libraries the package embeds, from the
/// `META-INF/<group>_<artifact>.version` markers AndroidX and friends write.
fn libraries(archive: &mut Archive, layout: &Layout) -> Vec<(String, String)> {
    let prefix = format!("{}META-INF/", layout.root);
    let markers: Vec<String> = archive
        .names_with_prefix(&prefix)
        .into_iter()
        .filter(|name| name.ends_with(".version"))
        .map(str::to_string)
        .collect();

    let mut libraries: Vec<(String, String)> = markers
        .into_iter()
        .filter_map(|entry| {
            let version = archive.read_text(&entry)?.trim().to_string();
            let marker = entry.strip_prefix(&prefix)?.strip_suffix(".version")?;
            let (group, artifact) = marker.split_once('_')?;
            // A few artifacts ship a marker holding an unresolved Gradle
            // placeholder instead of a version; those say nothing useful.
            is_version(&version).then(|| (format!("{}:{}", group, artifact), version))
        })
        .collect();
    libraries.sort();
    libraries
}

fn native_libraries(archive: &Archive, layout: &Layout) -> Vec<String> {
    let prefix = format!("{}lib/", layout.module);
    let mut names: Vec<String> = archive
        .names_with_prefix(&prefix)
        .into_iter()
        .filter_map(|name| name.rsplit('/').next())
        .filter(|name| name.ends_with(".so"))
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    names
}

fn is_version(value: &str) -> bool {
    value.starts_with(|c: char| c.is_ascii_digit()) && !value.contains(char::is_whitespace)
}

fn property(properties: &str, key: &str) -> Option<String> {
    properties
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(name, _)| name.trim() == key)
        .map(|(_, value)| value.trim().to_string())
}

fn text_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    current.as_str().map(str::to_string)
}
