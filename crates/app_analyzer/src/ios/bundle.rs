use crate::archive::Archive;
use crate::macho::MachOInfo;
use crate::{json_util, linking, macho, plist_util, provisioning, sdks};
use fastforge_core::AnalyzeError;
use plist::Dictionary;
use serde_json::{Map, Value, json};

/// How much of the app binary to read. Only the header and load commands are
/// needed, and they sit at the front of the slice — this bound keeps a large
/// universal binary from being pulled into memory whole.
const MAX_EXECUTABLE_BYTES: u64 = 32 * 1024 * 1024;

/// The `.app` inside an IPA, located under `Payload/`.
pub struct AppBundle {
    /// Prefix of the bundle inside the archive, e.g. `Payload/Example.app/`.
    pub prefix: String,
    pub info: Dictionary,
}

/// Finds the app bundle an IPA ships and reads its `Info.plist`.
pub fn find(archive: &mut Archive) -> Result<AppBundle, AnalyzeError> {
    let prefix = archive
        .entries()
        .iter()
        .filter_map(|entry| {
            let (bundle, rest) = entry.name.split_once(".app/")?;
            // Extensions and watch apps are nested deeper inside the payload.
            (rest == "Info.plist" && bundle.starts_with("Payload/") && !bundle[8..].contains('/'))
                .then(|| format!("{}.app/", bundle))
        })
        .min()
        .ok_or_else(|| AnalyzeError::Parse("Can't parse .ipa file.".to_string()))?;

    let info = archive
        .read_bytes(&format!("{}Info.plist", prefix))
        .and_then(|bytes| plist_util::parse_dictionary(&bytes))
        .ok_or_else(|| {
            AnalyzeError::Parse("Failed to parse Info.plist inside the .ipa".to_string())
        })?;

    Ok(AppBundle { prefix, info })
}

/// Everything the app declares about itself in `Info.plist`.
pub fn declared_metadata(info: &Dictionary) -> Map<String, Value> {
    let mut metadata = Map::new();
    json_util::insert_text(
        &mut metadata,
        "displayName",
        plist_util::text(info, "CFBundleDisplayName"),
    );
    json_util::insert_text(
        &mut metadata,
        "bundleName",
        plist_util::text(info, "CFBundleName"),
    );
    json_util::insert_text(
        &mut metadata,
        "executable",
        plist_util::text(info, "CFBundleExecutable"),
    );
    json_util::insert_text(
        &mut metadata,
        "minOSVersion",
        plist_util::text(info, "MinimumOSVersion"),
    );
    json_util::insert_text_array(
        &mut metadata,
        "deviceFamilies",
        device_families(info).into(),
    );
    json_util::insert_text_array(
        &mut metadata,
        "supportedPlatforms",
        plist_util::text_array(info, "CFBundleSupportedPlatforms"),
    );
    json_util::insert_text(
        &mut metadata,
        "category",
        plist_util::text(info, "LSApplicationCategoryType"),
    );
    json_util::insert_text(
        &mut metadata,
        "developmentRegion",
        plist_util::text(info, "CFBundleDevelopmentRegion"),
    );
    metadata
}

/// `UIDeviceFamily` is a list of numbers; the names are what people read.
fn device_families(info: &Dictionary) -> Vec<String> {
    let Some(families) = info
        .get("UIDeviceFamily")
        .and_then(|value| value.as_array())
    else {
        return Vec::new();
    };
    families
        .iter()
        .filter_map(|value| value.as_unsigned_integer())
        .map(|family| {
            match family {
                1 => "iPhone",
                2 => "iPad",
                3 => "Apple TV",
                4 => "Apple Watch",
                6 => "Mac",
                7 => "Apple Vision",
                _ => "unknown",
            }
            .to_string()
        })
        .collect()
}

/// What the app registers with the system and which permissions it asks for.
pub fn capabilities(info: &Dictionary) -> Map<String, Value> {
    let mut capabilities = Map::new();
    json_util::insert_array(&mut capabilities, "urlSchemes", url_schemes(info));
    json_util::insert_array(&mut capabilities, "documentTypes", document_types(info));
    json_util::insert_text_array(
        &mut capabilities,
        "backgroundModes",
        plist_util::text_array(info, "UIBackgroundModes"),
    );
    json_util::insert_text_array(
        &mut capabilities,
        "requiredDeviceCapabilities",
        plist_util::text_array(info, "UIRequiredDeviceCapabilities"),
    );
    json_util::insert_text_array(
        &mut capabilities,
        "supportedOrientations",
        plist_util::text_array(info, "UISupportedInterfaceOrientations"),
    );
    json_util::insert_flag(
        &mut capabilities,
        "allowsArbitraryLoads",
        info.get("NSAppTransportSecurity")
            .and_then(|value| value.as_dictionary())
            .and_then(|ats| plist_util::flag(ats, "NSAllowsArbitraryLoads")),
    );
    json_util::insert_flag(
        &mut capabilities,
        "encryptionExempt",
        plist_util::flag(info, "ITSAppUsesNonExemptEncryption").map(|uses| !uses),
    );
    json_util::insert_object(
        &mut capabilities,
        "privacyUsageDescriptions",
        privacy_usage_descriptions(info),
    );
    capabilities
}

fn url_schemes(info: &Dictionary) -> Vec<Value> {
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
            json_util::insert_text_array(
                &mut item,
                "schemes",
                plist_util::text_array(entry, "CFBundleURLSchemes"),
            );
            Value::Object(item)
        })
        .collect()
}

fn document_types(info: &Dictionary) -> Vec<Value> {
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
            json_util::insert_text_array(
                &mut item,
                "contentTypes",
                plist_util::text_array(entry, "LSItemContentTypes"),
            );
            Value::Object(item)
        })
        .collect()
}

fn privacy_usage_descriptions(info: &Dictionary) -> Map<String, Value> {
    info.iter()
        .filter(|(key, _)| key.ends_with("UsageDescription"))
        .filter_map(|(key, value)| {
            Some((key.clone(), Value::String(value.as_string()?.to_string())))
        })
        .collect()
}

/// Toolchain the app was built with, from the `DT*` keys Xcode writes.
pub fn build_info(info: &Dictionary) -> Map<String, Value> {
    let mut build = Map::new();
    for (key, source) in [
        ("sdk", "DTSDKName"),
        ("sdkBuild", "DTSDKBuild"),
        ("platformName", "DTPlatformName"),
        ("platformVersion", "DTPlatformVersion"),
        ("platformBuild", "DTPlatformBuild"),
        ("xcode", "DTXcode"),
        ("xcodeBuild", "DTXcodeBuild"),
        ("compiler", "DTCompiler"),
        ("machineOSBuild", "BuildMachineOSBuild"),
    ] {
        json_util::insert_text(&mut build, key, plist_util::text(info, source));
    }
    build
}

/// Reads the app's main executable out of the archive.
pub fn read_executable(archive: &mut Archive, bundle: &AppBundle) -> Option<MachOInfo> {
    let executable = plist_util::text(&bundle.info, "CFBundleExecutable")?;
    let bytes = archive.read_bytes_capped(
        &format!("{}{}", bundle.prefix, executable),
        MAX_EXECUTABLE_BYTES,
    )?;
    macho::inspect_bytes(&bytes)
}

/// Frameworks, app extensions and companion watch apps carried by the IPA.
pub fn components(archive: &mut Archive, bundle: &AppBundle) -> Map<String, Value> {
    let mut components = Map::new();
    json_util::insert_array(
        &mut components,
        "frameworks",
        nested_bundles(
            archive,
            &format!("{}Frameworks/", bundle.prefix),
            ".framework",
        ),
    );
    json_util::insert_text_array(
        &mut components,
        "libraries",
        Some(top_level_names(
            archive,
            &format!("{}Frameworks/", bundle.prefix),
            ".dylib",
        )),
    );
    json_util::insert_array(
        &mut components,
        "appExtensions",
        nested_bundles(archive, &format!("{}PlugIns/", bundle.prefix), ".appex"),
    );
    json_util::insert_array(
        &mut components,
        "watchApps",
        nested_bundles(archive, &format!("{}Watch/", bundle.prefix), ".app"),
    );
    components
}

/// Describes each nested bundle of the given kind, reading its own `Info.plist`
/// for the identity it reports to the system.
fn nested_bundles(archive: &mut Archive, prefix: &str, extension: &str) -> Vec<Value> {
    let names = top_level_names(archive, prefix, extension);

    names
        .into_iter()
        .map(|name| {
            let bundle_prefix = format!("{}{}/", prefix, name);
            let mut item = Map::new();
            item.insert("name".to_string(), Value::String(name));
            item.insert(
                "sizeBytes".to_string(),
                json!(archive.size_under(&bundle_prefix)),
            );
            if let Some(info) = archive
                .read_bytes(&format!("{}Info.plist", bundle_prefix))
                .and_then(|bytes| plist_util::parse_dictionary(&bytes))
            {
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
                    "extensionPoint",
                    info.get("NSExtension")
                        .and_then(|value| value.as_dictionary())
                        .and_then(|extension| {
                            plist_util::text(extension, "NSExtensionPointIdentifier")
                        }),
                );
            }
            Value::Object(item)
        })
        .collect()
}

/// Names of the entries directly under `prefix` that end in `extension`.
fn top_level_names(archive: &Archive, prefix: &str, extension: &str) -> Vec<String> {
    let mut names: Vec<String> = archive
        .names_with_prefix(prefix)
        .into_iter()
        .filter_map(|name| name[prefix.len()..].split('/').next())
        .filter(|name| name.ends_with(extension))
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Describes what the app is built with, combining the payload layout with the
/// libraries its binary links against.
pub fn tech_stack(
    archive: &mut Archive,
    bundle: &AppBundle,
    executable: Option<&MachOInfo>,
) -> Map<String, Value> {
    let mut stack = Map::new();

    let frameworks = top_level_names(
        archive,
        &format!("{}Frameworks/", bundle.prefix),
        ".framework",
    );
    let runtime = detect_runtime(archive, bundle, &frameworks);
    stack.insert(
        "runtime".to_string(),
        Value::String(runtime.kind.to_string()),
    );
    json_util::insert_object(&mut stack, runtime.kind, runtime.details);

    let links = executable
        .map(|executable| linking::classify(&executable.libraries))
        .unwrap_or_default();
    json_util::insert_text_array(
        &mut stack,
        "languages",
        Some(linking::languages(&links, executable)),
    );
    json_util::insert_text_array(&mut stack, "uiToolkits", Some(linking::ui_toolkits(&links)));
    if let Some(executable) = executable {
        json_util::insert_object(&mut stack, "toolchain", linking::toolchain(executable));
    }
    json_util::insert_array(
        &mut stack,
        "thirdPartySdks",
        sdks::recognize(
            links
                .embedded_frameworks
                .iter()
                .chain(links.embedded_libraries.iter())
                .chain(frameworks.iter()),
        ),
    );
    linking::insert_sections(&mut stack, links);
    stack
}

struct Runtime {
    kind: &'static str,
    details: Map<String, Value>,
}

fn detect_runtime(archive: &Archive, bundle: &AppBundle, frameworks: &[String]) -> Runtime {
    let prefix = bundle.prefix.as_str();
    let has_framework = |name: &str| frameworks.iter().any(|framework| framework == name);

    if has_framework("Flutter.framework")
        || archive.contains_prefix(&format!("{}flutter_assets/", prefix))
    {
        let assets = format!("{}flutter_assets/", prefix);
        let mut flutter = Map::new();
        if archive.contains_prefix(&assets) {
            flutter.insert(
                "assets".to_string(),
                json!({
                    "fileCount": archive.count_under(&assets),
                    "sizeBytes": archive.size_under(&assets),
                }),
            );
        }
        // Release builds compile Dart ahead of time into App.framework; debug
        // builds ship the kernel blob with the assets instead.
        flutter.insert(
            "aot".to_string(),
            Value::Bool(!archive.contains(&format!("{}kernel_blob.bin", assets))),
        );
        // Plugins ship as frameworks named after their pub package, which is
        // `lowercase_with_underscores` by convention.
        json_util::insert_text_array(
            &mut flutter,
            "plugins",
            Some(
                frameworks
                    .iter()
                    .filter_map(|name| name.strip_suffix(".framework"))
                    .filter(|name| {
                        name.chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                    })
                    .map(str::to_string)
                    .collect(),
            ),
        );
        return Runtime {
            kind: "flutter",
            details: flutter,
        };
    }

    if archive.contains(&format!("{}main.jsbundle", prefix)) || has_framework("React.framework") {
        let mut react_native = Map::new();
        react_native.insert(
            "jsEngine".to_string(),
            Value::String(
                if has_framework("hermes.framework") || has_framework("hermes_engine.framework") {
                    "hermes".to_string()
                } else {
                    "jsc".to_string()
                },
            ),
        );
        react_native.insert(
            "bundled".to_string(),
            Value::Bool(archive.contains(&format!("{}main.jsbundle", prefix))),
        );
        return Runtime {
            kind: "react-native",
            details: react_native,
        };
    }

    if has_framework("UnityFramework.framework")
        || archive.contains_prefix(&format!("{}Data/Managed/", prefix))
    {
        return Runtime {
            kind: "unity",
            details: Map::new(),
        };
    }
    if archive.contains(&format!("{}www/index.html", prefix)) {
        return Runtime {
            kind: "cordova",
            details: Map::new(),
        };
    }

    Runtime {
        kind: "native",
        details: Map::new(),
    }
}

/// The provisioning profile the IPA was signed with, if it carries one.
pub fn provisioning_profile(archive: &mut Archive, bundle: &AppBundle) -> Option<Value> {
    let bytes = archive.read_bytes(&format!("{}embedded.mobileprovision", bundle.prefix))?;
    provisioning::parse(&bytes)
}
