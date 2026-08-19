use crate::json_util;
use crate::macho::MachOInfo;
use serde_json::{Map, Value};

/// Where each linked library comes from.
///
/// Apple's platforms share install-name conventions, so the same classification
/// serves a macOS bundle and an iOS app.
#[derive(Default)]
pub struct LinkedSummary {
    /// Whether the binary pulls in the Swift runtime shims.
    pub swift_runtime: bool,
    pub system_frameworks: Vec<String>,
    pub private_frameworks: Vec<String>,
    pub system_libraries: Vec<String>,
    pub embedded_frameworks: Vec<String>,
    pub embedded_libraries: Vec<String>,
    pub other_libraries: Vec<String>,
}

/// Sorts each linked install name into where it comes from: the OS, a private
/// Apple framework, or the bundle itself.
pub fn classify(libraries: &[String]) -> LinkedSummary {
    let mut summary = LinkedSummary::default();

    for path in libraries.iter().map(String::as_str) {
        // Every Swift app links a long tail of `/usr/lib/swift/libswift*.dylib`
        // shims. They say nothing beyond "this is Swift", which `languages`
        // already reports, so they are collapsed into a single flag.
        if path.starts_with("/usr/lib/swift/") {
            summary.swift_runtime = true;
            continue;
        }

        let bucket = if let Some(rest) = path.strip_prefix("/System/Library/Frameworks/") {
            (&mut summary.system_frameworks, framework_name(rest))
        } else if let Some(rest) = path.strip_prefix("/System/Library/PrivateFrameworks/") {
            (&mut summary.private_frameworks, framework_name(rest))
        } else if path.starts_with("/usr/lib/") {
            (&mut summary.system_libraries, last_component(path))
        } else if path.starts_with('@') {
            if path.contains(".framework/") {
                (&mut summary.embedded_frameworks, framework_name(path))
            } else {
                (&mut summary.embedded_libraries, last_component(path))
            }
        } else if path.contains(".framework/") {
            (&mut summary.other_libraries, framework_name(path))
        } else {
            (&mut summary.other_libraries, last_component(path))
        };

        let (bucket, name) = bucket;
        if let Some(name) = name
            && !bucket.contains(&name)
        {
            bucket.push(name);
        }
    }

    for bucket in [
        &mut summary.system_frameworks,
        &mut summary.private_frameworks,
        &mut summary.system_libraries,
        &mut summary.embedded_frameworks,
        &mut summary.embedded_libraries,
        &mut summary.other_libraries,
    ] {
        bucket.sort();
    }
    summary
}

/// Languages inferred from the runtime libraries the binary pulls in.
pub fn languages(links: &LinkedSummary, macho: Option<&MachOInfo>) -> Vec<String> {
    let uses_swift = links.swift_runtime
        || links
            .system_libraries
            .iter()
            .any(|name| name.starts_with("libswift"))
        || macho.is_some_and(|macho| macho.tools.iter().any(|tool| tool.name == "swift"));
    let uses_objc = links
        .system_libraries
        .iter()
        .any(|name| name.starts_with("libobjc"));
    let uses_cpp = links
        .system_libraries
        .iter()
        .any(|name| name.starts_with("libc++") || name.starts_with("libstdc++"));

    [
        uses_swift.then(|| "Swift".to_string()),
        uses_objc.then(|| "Objective-C".to_string()),
        uses_cpp.then(|| "C++".to_string()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub fn ui_toolkits(links: &LinkedSummary) -> Vec<String> {
    ["SwiftUI", "AppKit", "UIKit"]
        .into_iter()
        .filter(|toolkit| links.system_frameworks.iter().any(|name| name == toolkit))
        .map(str::to_string)
        .collect()
}

/// Platform, deployment target and the compilers recorded in the binary. These
/// come from the build itself, unlike the `Info.plist` declarations.
pub fn toolchain(macho: &MachOInfo) -> Map<String, Value> {
    let mut toolchain = Map::new();
    json_util::insert_text(
        &mut toolchain,
        "platform",
        macho.platform.map(str::to_string),
    );
    json_util::insert_text(&mut toolchain, "minOS", macho.min_os.clone());
    json_util::insert_text(&mut toolchain, "sdk", macho.sdk.clone());
    for tool in &macho.tools {
        json_util::insert_text(&mut toolchain, tool.name, tool.version.clone());
    }
    toolchain
}

/// Adds the link-derived sections shared by the Apple platforms.
pub fn insert_sections(stack: &mut Map<String, Value>, links: LinkedSummary) {
    json_util::insert_text_array(stack, "systemFrameworks", Some(links.system_frameworks));
    json_util::insert_text_array(stack, "privateFrameworks", Some(links.private_frameworks));
    json_util::insert_text_array(stack, "systemLibraries", Some(links.system_libraries));
    json_util::insert_text_array(stack, "embeddedFrameworks", Some(links.embedded_frameworks));
    json_util::insert_text_array(stack, "embeddedLibraries", Some(links.embedded_libraries));
    json_util::insert_text_array(stack, "otherLibraries", Some(links.other_libraries));
}

/// Framework base name from any path that contains `<name>.framework`.
pub fn framework_name(path: &str) -> Option<String> {
    path.split('/')
        .find_map(|component| component.strip_suffix(".framework"))
        .map(str::to_string)
}

pub fn last_component(path: &str) -> Option<String> {
    path.rsplit('/').next().map(str::to_string)
}
