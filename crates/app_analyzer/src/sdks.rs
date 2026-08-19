use serde_json::{Value, json};

/// Third-party SDKs worth naming when they turn up in an app. The list is
/// deliberately short: it covers the libraries that change how an app is
/// released (updaters, crash reporting, analytics) plus a few very common
/// ones. Anything not listed still shows up under the linked-library keys.
const KNOWN_SDKS: &[(&str, &str)] = &[
    ("Sparkle", "updater"),
    ("Squirrel", "updater"),
    ("Sentry", "crash-reporting"),
    ("Bugsnag", "crash-reporting"),
    ("Crashlytics", "crash-reporting"),
    ("FirebaseCrashlytics", "crash-reporting"),
    ("FirebaseAnalytics", "analytics"),
    ("AppCenter", "analytics"),
    ("AppCenterAnalytics", "analytics"),
    ("AppCenterCrashes", "crash-reporting"),
    ("HockeySDK", "analytics"),
    ("Realm", "database"),
    ("RealmSwift", "database"),
    ("GRDB", "database"),
    ("Alamofire", "networking"),
    ("ReactiveObjC", "reactive"),
    ("ReactiveCocoa", "reactive"),
    ("RxSwift", "reactive"),
    ("RxCocoa", "reactive"),
    ("Mantle", "model-mapping"),
    ("Lottie", "animation"),
    ("KeychainAccess", "security"),
    ("MASShortcut", "hotkeys"),
    ("KeyboardShortcuts", "hotkeys"),
    ("HotKey", "hotkeys"),
];

/// Picks the recognizable SDKs out of the component names an app ships.
///
/// The same SDK reaches us bare from a link table and with its extension from a
/// directory listing, so names are normalized before matching and the result is
/// reported under the plain SDK name.
pub fn recognize<'a>(names: impl Iterator<Item = &'a String>) -> Vec<Value> {
    let mut found: Vec<(&str, &'static str)> = names
        .map(|name| normalize(name))
        .filter_map(|name| Some((name, category_of(name)?)))
        .collect();
    found.sort_unstable();
    found.dedup();

    found
        .into_iter()
        .map(|(name, category)| json!({ "name": name, "category": category }))
        .collect()
}

fn normalize(name: &str) -> &str {
    name.strip_suffix(".framework")
        .or_else(|| name.strip_suffix(".dylib"))
        .unwrap_or(name)
}

fn category_of(name: &str) -> Option<&'static str> {
    if let Some((_, category)) = KNOWN_SDKS.iter().find(|(sdk, _)| *sdk == name) {
        return Some(category);
    }
    name.starts_with("Firebase").then_some("firebase")
}
