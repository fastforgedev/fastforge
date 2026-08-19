//! Infers the target platform when `--platform` is omitted.
//!
//! Resolution order:
//! 1. The package/build target(s): most targets map to exactly one platform
//!    (e.g. `dmg` → macos, `apk` → android). Multi-target invocations use the
//!    intersection of every target's candidate set.
//! 2. Ambiguous targets (`zip`, `direct`, `custom`) or no target at all fall
//!    back to the project layout (which platform directories/markers exist)
//!    plus what the host OS can actually build, preferring the host platform
//!    when several candidates remain.

use anyhow::{Result, anyhow};
use fastforge_app_builder::Platform;
use std::path::Path;

/// Platforms a package/build target can belong to, mirroring the
/// `resolve_packager` routing matrix.
fn target_candidates(target: &str) -> &'static [Platform] {
    match target {
        "aab" | "apk" => &[Platform::Android],
        "ipa" => &[Platform::IOS],
        "appimage" | "deb" | "pacman" | "rpm" => &[Platform::Linux],
        "dmg" | "pkg" => &[Platform::MacOS],
        "app" | "hap" => &[Platform::Ohos],
        "exe" | "msix" => &[Platform::Windows],
        "zip" => &[
            Platform::Linux,
            Platform::MacOS,
            Platform::Web,
            Platform::Windows,
        ],
        "direct" => &[Platform::Linux, Platform::Web, Platform::Windows],
        // `custom` (and anything unknown) carries no platform information;
        // unknown targets are rejected later by `resolve_packager`.
        _ => Platform::all(),
    }
}

/// Whether the host OS is able to build for `platform`. Desktop platforms are
/// host-locked; android/web/ohos build anywhere; ios needs a macOS host.
fn buildable_on_host(platform: Platform, host: Option<Platform>) -> bool {
    match platform {
        Platform::MacOS | Platform::IOS => host == Some(Platform::MacOS),
        Platform::Windows => host == Some(Platform::Windows),
        Platform::Linux => host == Some(Platform::Linux),
        Platform::Android | Platform::Web | Platform::Ohos => true,
    }
}

/// Platforms the project in `root` is set up for.
///
/// Flutter projects (with a `pubspec.yaml`) declare platforms via their
/// platform directories. Native projects are recognized by their build-system
/// markers: gradle files → android, an Xcode project → macos/ios.
fn project_candidates(root: &Path) -> Vec<Platform> {
    if root.join("pubspec.yaml").is_file() {
        return Platform::all()
            .iter()
            .copied()
            .filter(|p| root.join(p.as_str()).is_dir())
            .collect();
    }

    let mut candidates = Vec::new();
    if root.join("gradlew").is_file()
        || root.join("app/build.gradle").is_file()
        || root.join("app/build.gradle.kts").is_file()
    {
        candidates.push(Platform::Android);
    }
    let has_xcodeproj = std::fs::read_dir(root).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|e| e.path().extension().is_some_and(|ext| ext == "xcodeproj"))
    });
    if has_xcodeproj {
        candidates.push(Platform::IOS);
        candidates.push(Platform::MacOS);
    }
    candidates
}

/// Infers the platform from the given package/build targets, the project in
/// the current directory, and the host OS.
pub fn infer_platform(targets: &[&str]) -> Result<Platform> {
    let platform = infer_platform_in(targets, Path::new("."), Platform::current())?;
    log::info!(
        "Auto-detected platform '{}' (pass --platform to override)",
        platform.as_str()
    );
    Ok(platform)
}

fn infer_platform_in(targets: &[&str], root: &Path, host: Option<Platform>) -> Result<Platform> {
    // 1. Intersect the candidate sets of every requested target.
    let mut candidates: Vec<Platform> = Platform::all().to_vec();
    for target in targets {
        let allowed = target_candidates(target);
        candidates.retain(|p| allowed.contains(p));
    }
    if candidates.is_empty() {
        return Err(anyhow!(
            "Targets `{}` do not share a common platform. \
             Run one `package` invocation per platform.",
            targets.join(",")
        ));
    }
    if let [platform] = candidates[..] {
        return Ok(platform);
    }

    // 2. Narrow the ambiguous set down by project layout and host OS.
    let project = project_candidates(root);
    candidates.retain(|p| project.contains(p) && buildable_on_host(*p, host));

    match candidates[..] {
        [platform] => Ok(platform),
        // Several candidates left (e.g. `zip` in a project with both a
        // desktop and a web directory): prefer the host platform.
        _ if host.is_some_and(|h| candidates.contains(&h)) => Ok(host.unwrap()),
        [] => Err(anyhow!(
            "Unable to detect the platform{}: no matching platform directory found \
             in this project. Please specify --platform explicitly.",
            fmt_targets(targets),
        )),
        _ => Err(anyhow!(
            "Unable to detect the platform{}: candidates are {}. \
             Please specify --platform explicitly.",
            fmt_targets(targets),
            candidates
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

fn fmt_targets(targets: &[&str]) -> String {
    if targets.is_empty() {
        String::new()
    } else {
        format!(" for target(s) `{}`", targets.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn flutter_project(platform_dirs: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("pubspec.yaml"), "name: app\n").unwrap();
        for platform in platform_dirs {
            std::fs::create_dir(dir.path().join(platform)).unwrap();
        }
        dir
    }

    #[test]
    fn unambiguous_targets_need_no_project() {
        let empty = TempDir::new().unwrap();
        for (target, expected) in [
            ("aab", Platform::Android),
            ("apk", Platform::Android),
            ("ipa", Platform::IOS),
            ("appimage", Platform::Linux),
            ("deb", Platform::Linux),
            ("pacman", Platform::Linux),
            ("rpm", Platform::Linux),
            ("dmg", Platform::MacOS),
            ("pkg", Platform::MacOS),
            ("app", Platform::Ohos),
            ("hap", Platform::Ohos),
            ("exe", Platform::Windows),
            ("msix", Platform::Windows),
        ] {
            let inferred = infer_platform_in(&[target], empty.path(), None)
                .unwrap_or_else(|e| panic!("target {target}: {e}"));
            assert_eq!(inferred, expected, "target {target}");
        }
    }

    #[test]
    fn multi_target_intersection() {
        let empty = TempDir::new().unwrap();
        // `zip` alone is ambiguous, but dmg+zip only fits macos.
        let inferred = infer_platform_in(&["dmg", "zip"], empty.path(), None).unwrap();
        assert_eq!(inferred, Platform::MacOS);
    }

    #[test]
    fn conflicting_targets_are_rejected() {
        let empty = TempDir::new().unwrap();
        let err = infer_platform_in(&["dmg", "apk"], empty.path(), None).unwrap_err();
        assert!(err.to_string().contains("common platform"), "{err}");
    }

    #[test]
    fn ambiguous_zip_resolved_by_project_dir() {
        let dir = flutter_project(&["web"]);
        let inferred =
            infer_platform_in(&["zip"], dir.path(), Some(Platform::MacOS)).unwrap();
        assert_eq!(inferred, Platform::Web);
    }

    #[test]
    fn ambiguous_zip_prefers_host_platform() {
        let dir = flutter_project(&["macos", "web", "windows", "linux"]);
        let inferred =
            infer_platform_in(&["zip"], dir.path(), Some(Platform::MacOS)).unwrap();
        assert_eq!(inferred, Platform::MacOS);
    }

    #[test]
    fn no_target_uses_project_and_host() {
        let dir = flutter_project(&["macos", "ios", "android"]);
        let inferred = infer_platform_in(&[], dir.path(), Some(Platform::MacOS)).unwrap();
        assert_eq!(inferred, Platform::MacOS);

        // On a linux host the same project can only build android.
        let dir = flutter_project(&["android"]);
        let inferred = infer_platform_in(&[], dir.path(), Some(Platform::Linux)).unwrap();
        assert_eq!(inferred, Platform::Android);
    }

    #[test]
    fn undetectable_platform_asks_for_flag() {
        let dir = flutter_project(&[]);
        let err = infer_platform_in(&["zip"], dir.path(), Some(Platform::MacOS)).unwrap_err();
        assert!(err.to_string().contains("--platform"), "{err}");
    }

    #[test]
    fn ambiguous_candidates_ask_for_flag() {
        // android + web both buildable on a linux host with no linux dir:
        // nothing to tie-break on, so the user must decide.
        let dir = flutter_project(&["android", "web"]);
        let err = infer_platform_in(&[], dir.path(), Some(Platform::MacOS)).unwrap_err();
        assert!(err.to_string().contains("--platform"), "{err}");
    }

    #[test]
    fn native_xcode_project_zip_is_macos() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("MyApp.xcodeproj")).unwrap();
        let inferred =
            infer_platform_in(&["zip"], dir.path(), Some(Platform::MacOS)).unwrap();
        assert_eq!(inferred, Platform::MacOS);
    }

    #[test]
    fn native_gradle_project_is_android() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("app")).unwrap();
        std::fs::write(dir.path().join("app/build.gradle.kts"), "").unwrap();
        let inferred = infer_platform_in(&[], dir.path(), Some(Platform::MacOS)).unwrap();
        assert_eq!(inferred, Platform::Android);
    }
}
