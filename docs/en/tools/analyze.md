# App Package Analysis

English | [简体中文](../../zh-Hans/tools/analyze.md)

`fastforge analyze` reads an application's identity from an artifact — name,
identifier, version and build number — and, for the formats listed below,
describes what the artifact is made of: the technology it is built on, the
libraries it ships, its size composition and how it is signed.

It accepts any number of artifacts, and scans directories for the packages
they contain. Results are printed as JSON, or rendered as a standalone HTML
report.

## Supported Formats

| Format        | Platform restrictions | Dependencies                                                              |
| ------------- | --------------------- | ------------------------------------------------------------------------- |
| APK           | No fixed host         | `aapt2` under `ANDROID_HOME`; optionally `apksigner`                      |
| AAB           | No fixed host         | `aapt2`, or `BUNDLETOOL`                                                  |
| IPA           | No fixed host         | No external tool                                                          |
| DMG           | macOS only            | `hdiutil`, `diskutil`; optionally `codesign`, `spctl`, `xcrun stapler`    |
| `.app` bundle | macOS only            | Local `Info.plist`; optionally `codesign`, `spctl`, `xcrun stapler`       |

## Write to the Terminal

```bash
fastforge analyze dist/app-release.apk
```

Every format reports the same identity at the top level, alongside the file's
own facts:

```json
{
  "platform": "android",
  "format": "apk",
  "identifier": "com.example.app",
  "name": "Example",
  "version": "1.0.0",
  "buildNumber": 1,
  "fileName": "app-release.apk",
  "sizeBytes": 55605086,
  "sha256": "3d60a82610b81ce760bc999f5f36917b41256a43184f401a747923c0c01c8a5e"
}
```

## Write to a File

```bash
fastforge analyze dist/app-release.apk \
  --output analysis.json
```

## Several Artifacts at Once

Every argument is either an artifact or a directory to scan. Directories are
walked recursively, skipping hidden entries and never following symlinks; a
`.app` bundle is treated as an artifact rather than a directory to descend
into.

```bash
fastforge analyze dist
fastforge analyze dist/android dist/macos build/ios/ipa
```

Analyzing more than one artifact wraps the results:

```json
{
  "generatedAt": "2026-08-04T13:06:12+08:00",
  "artifactCount": 2,
  "artifacts": [{ "platform": "android", "format": "apk", "...": "..." }],
  "failures": [{ "path": "dist/broken.apk", "error": "Not a readable Android package" }]
}
```

Naming a single artifact keeps returning just that artifact's payload, as
before. A path you name has to analyze successfully or the command fails; an
artifact merely found while scanning is recorded under `failures` so one bad
file does not sink the whole run.

## HTML Report

```bash
fastforge analyze dist --output report.html
```

The report is a single self-contained file — no external styles, scripts or
fonts — so it can be opened straight from disk or attached to a build.

It opens with a summary row and three distributions: artifacts per runtime,
per signing state, and the largest artifacts. Clicking a bar filters
everything below it, and the filters stack. The table sorts by any column,
and a row expands in place to show that artifact's identity, tech stack, size
composition, signing, and the full analysis JSON. Anything that failed to
analyze is listed at the end.

The page renders itself from the analysis embedded in it, so it needs
JavaScript; the JSON output is the equivalent for anything that reads the
results programmatically.

`--output report.html` selects HTML from the file extension; pass
`--format html` to write it to stdout instead, or `--format json` to keep JSON
for a `.html` output path.

## Tech Stack

Every deeply analyzed format reports a `techStack` section describing what the
app is built with:

| Field                            | Content                                                                                          |
| -------------------------------- | ------------------------------------------------------------------------------------------------ |
| `runtime`                        | `flutter`, `electron`, `react-native`, `unity`, `cordova`, `dotnet`, `qt`, `java` or `native`      |
| `<runtime>`                       | Details that runtime exposes — engine revision, build mode, AOT, plugins, JS engine, asar manifest |
| `languages`                      | Inferred from the runtimes the binary links or the payload carries                                 |
| `uiToolkits`                     | SwiftUI / AppKit / UIKit on Apple platforms, Jetpack Compose / AppCompat on Android                |
| `toolchain` / `buildTools`       | Platform, deployment target, SDK, and the compiler, linker or Gradle versions recorded in the build |
| `libraries` / `dependencies`     | Maven coordinates and versions an Android package embeds                                           |
| `systemFrameworks`, `embeddedFrameworks`, `systemLibraries`, `privateFrameworks` | What an Apple binary links against         |
| `nativeLibraries`                | The `.so` files an Android package ships                                                          |
| `thirdPartySdks`                 | Recognized SDKs and what they do — updaters, crash reporting, analytics…                          |

Apple link tables come from the main executable's Mach-O load commands, so they
describe what the app itself links; code reached only through an embedded
framework appears under that framework instead.

## macOS: `.app` and DMG

Beyond `techStack`, a bundle reports its architectures (read from the Mach-O
header), size composition, embedded components and signing state.

| Field                                                     | Content                                                                        |
| --------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `architectures`, `universal`                              | Slices in the executable — `arm64`, `arm64e`, `x86_64`, …                       |
| `sizeBytes`, `fileCount`, `sizeBreakdown`, `largestFiles` | Bundle size, size per directory under `Contents`, the ten biggest files          |
| `buildInfo`                                               | SDK, platform, Xcode and compiler stamps from `Info.plist`                       |
| `components`                                              | Embedded frameworks, libraries, helper apps, XPC services and plug-ins           |
| `codeSignature`                                           | Signing type, team, authority chain, hardened runtime, entitlements, notarization, Gatekeeper |
| `provisioningProfile`                                     | Name, team, distribution type, expiration                                        |
| `urlSchemes`, `documentTypes`, `privacyUsageDescriptions` | What the app registers and which permissions it prompts for                      |
| `category`, `minOSVersion`, `localizations`, `sandboxed`  | Distribution metadata declared by the bundle                                     |

A DMG additionally reports the image itself, and nests the bundle analysis under
`app`:

| Field           | Content                                                                                       |
| --------------- | ---------------------------------------------------------------------------------------------- |
| `codeSignature` | The image's own signature and notarization state                                                 |
| `diskImage`     | Format (`UDZO`, `ULFO`, …), compression, checksum, partitions                                     |
| `volume`        | Volume name, contents, `/Applications` shortcut, custom window layout, background, volume icon    |
| `app` / `apps`  | Full analysis of the primary bundle, plus a summary of the others when the image ships several   |

```bash
fastforge analyze dist/1.0.0+1/example-1.0.0+1-macos.dmg
```

```json
{
  "platform": "macos",
  "format": "dmg",
  "identifier": "com.example.app",
  "version": "1.0.0",
  "sha256": "83bc18419eab947f614e4d3aeb98daa0db9c77365f2c1de2141d76b98375946c",
  "diskImage": { "format": "UDZO", "compressed": true, "encrypted": false },
  "volume": { "name": "Example", "hasApplicationsSymlink": true },
  "app": {
    "architectures": ["x86_64", "arm64"],
    "techStack": {
      "runtime": "flutter",
      "languages": ["Swift", "Objective-C"],
      "toolchain": { "platform": "macOS", "minOS": "12.0", "sdk": "26.5" }
    },
    "codeSignature": { "signingType": "developer-id", "notarization": { "stapled": true } }
  }
}
```

## iOS: IPA

The app bundle is read straight out of the archive — no unpacking, and no
macOS host required.

| Field                                                       | Content                                                                       |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `architectures`, `minOSVersion`, `deviceFamilies`           | Slices in the app binary, deployment target, iPhone / iPad / Vision support     |
| `buildInfo`                                                 | SDK, Xcode and compiler stamps from `Info.plist`                                |
| `components`                                                | Embedded frameworks, app extensions (with their extension point), watch apps    |
| `capabilities`                                              | URL schemes, document types, background modes, required capabilities, ATS, privacy usage descriptions |
| `provisioningProfile`                                       | Name, team, expiration, entitlements, and the distribution type — `development`, `ad-hoc`, `enterprise` or `app-store` |
| `contents`                                                  | Entry count, size per directory, the ten biggest entries                        |
| `codeSignature`                                             | Whether the payload carries a sealed resource directory                         |

```json
{
  "platform": "ios",
  "format": "ipa",
  "identifier": "dev.example.app",
  "deviceFamilies": ["iPhone", "iPad"],
  "architectures": ["arm64"],
  "techStack": {
    "runtime": "flutter",
    "flutter": { "aot": true, "plugins": ["url_launcher_ios"] },
    "uiToolkits": ["SwiftUI", "UIKit"],
    "toolchain": { "platform": "iOS", "minOS": "15.0", "sdk": "17.2", "swift": "5.9" }
  },
  "provisioningProfile": { "distributionType": "ad-hoc", "provisionedDeviceCount": 3 }
}
```

## Android: APK and AAB

`aapt2` (or `bundletool`) supplies the manifest; everything else is read from the
package itself.

| Field        | Content                                                                                              |
| ------------ | ----------------------------------------------------------------------------------------------------- |
| `abis`       | ABIs the package ships native code for                                                                 |
| `manifest`   | min / target / compile SDK, permissions, features, launchable activity, locales, densities, screens     |
| `techStack`  | Runtime, languages, UI toolkit, AGP / Gradle / Kotlin versions, AndroidX libraries with versions, native libraries |
| `contents`   | Entry count, dex count and size, size per directory, the ten biggest entries                            |
| `signature`  | APK: verified schemes and certificates via `apksigner`. AAB: whether it is JAR-signed                   |
| `modules`    | AAB only — the base module and each dynamic feature, with size and content                              |

An app bundle also records the dependency graph the build resolved, which is
richer than the version markers an APK carries:

```json
{
  "platform": "android",
  "format": "aab",
  "abis": ["arm64-v8a"],
  "manifest": { "minSdkVersion": 24, "targetSdkVersion": 35 },
  "techStack": {
    "runtime": "flutter",
    "languages": ["Kotlin", "Dart", "C/C++"],
    "buildTools": { "androidGradlePlugin": "8.7.2", "gradle": "8.9", "kotlin": "2.1.0" },
    "dependencies": [{ "name": "androidx.core:core", "version": "1.17.0" }]
  },
  "modules": [{ "name": "base", "dexCount": 1 }, { "name": "premium", "dexCount": 1 }]
}
```

`contents.sizeBreakdown` uses compressed sizes, since that is what the download
costs; `largestEntries` reports both compressed and uncompressed.

## bundletool Fallback for AAB

If no working `aapt2` can be found, point `BUNDLETOOL` to a bundletool JAR:

```bash
export BUNDLETOOL=/path/to/bundletool.jar
fastforge analyze dist/app-release.aab
```

## CI Usage

```bash
fastforge analyze "$ARTIFACT" --output artifact-metadata.json
```

The command exits with a nonzero status for unsupported extensions, missing
tools, or artifacts that cannot be parsed.

## Notes

- Keys are omitted rather than set to `null` when an artifact does not carry
  that metadata, so the shape varies with the input.
- `buildNumber` is a string for Apple artifacts, since `CFBundleVersion` is not
  always numeric; it stays an integer for Android version codes.
- macOS signature inspection needs `codesign`; Gatekeeper (`spctl`) and
  notarization (`xcrun stapler`) are only checked for signed artifacts and may
  reach the network. Every external tool call is capped at 30 seconds, and a
  missing tool simply leaves its section out.
- Encrypted disk images are rejected, because attaching one would block on a
  password prompt.
