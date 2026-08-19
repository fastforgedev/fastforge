# fastforge_app_analyzer

A unified application package analyzer — reads metadata from compiled app packages (APK, IPA, AAB, DMG, etc.).

## Status

Work in progress (WIP). Supports analyzing Android (APK, AAB), iOS (IPA) and macOS (DMG, `.app`) packages.

## Supported Formats

| Format | Analyzer | Status |
|---|---|---|
| `android/apk` | `AndroidApkAnalyzer` | ✅ Implemented |
| `android/aab` | `AndroidAabAnalyzer` | ✅ Implemented |
| `ios/ipa` | `IOSIpaAnalyzer` | ✅ Implemented |
| `macos/dmg` | `MacOSDmgAnalyzer` | ✅ Implemented |
| `macos/app` | `MacOSAppAnalyzer` | ✅ Implemented |

Every analyzer reports the app's identity (`identifier`, `name`, `version`,
`buildNumber`) and then describes what the artifact is made of: the detected
runtime (Flutter, Electron, React Native, Unity, …), languages and UI toolkit,
the toolchain that built it, the libraries it ships, its size composition and
how it is signed. Per format, that also covers the disk image and volume of a
DMG, the provisioning profile and app extensions of an IPA, and the manifest,
modules and dependency graph of an Android package.
See [docs/en/tools/analyze.md](../../docs/en/tools/analyze.md) for the fields.

## API Usage

```rust
use fastforge_app_analyzer::MacOSAppAnalyzer;
use fastforge_core::{AnalyzeConfig, AppAnalyzer};

fn analyze_app() -> anyhow::Result<()> {
    let analyzer = MacOSAppAnalyzer::new();
    let config = AnalyzeConfig::new("path/to/Example.app".to_string());
    let result = analyzer.analyze(config)?;

    // `data` is a `serde_json::Value` holding the analysis payload.
    println!("{}", serde_json::to_string_pretty(&result.data)?);

    Ok(())
}
```

## Run Tests

```bash
cargo test -p fastforge_app_analyzer --offline
```
