# Xcode Builder

English | [简体中文](../../zh-Hans/builders/xcode.md)

Xcode Builder supports native Xcode projects (projects without `pubspec.yaml`):

- **iOS and macOS** use Apple's `xcodebuild` on macOS with the Xcode command-line tools installed.
- **Windows, Linux, Web (WebAssembly), Android, and HarmonyOS (`ohos`)** use [CrossXcodeBuild](https://github.com/libnativeapi/crossswift) (`libnativeapi/crossswift`) to compile unmodified `.xcodeproj` / `.xcworkspace` projects for non-Apple targets.

## Current Integration

| Platform          | Toolchain       | Allowed build hosts | Build result                       | Subsequent packaging                                                 |
| ----------------- | --------------- | ------------------- | ---------------------------------- | -------------------------------------------------------------------- |
| iOS               | Apple xcodebuild | macOS               | `.xcarchive`, IPA                  | IPA packager                                                         |
| macOS             | Apple xcodebuild | macOS               | `.app`                             | DMG, PKG, ZIP, `tar.gz`, and `custom` packagers                      |
| Windows           | CrossXcodeBuild | Windows             | Staged app directory (`.exe`, DLLs) | EXE, MSIX, ZIP, `tar.gz`, `direct`, and `custom` packagers           |
| Linux             | CrossXcodeBuild | Linux               | Staged app directory (`lib/*.so`)   | DEB, RPM, Pacman, AppImage, Flatpak, Nix, ZIP, `tar.gz`, `direct`, `custom` |
| Web (WebAssembly) | CrossXcodeBuild | macOS, Linux        | Staged web directory (`index.html`, `.wasm`) | ZIP, `tar.gz`, `direct`, and `custom` packagers             |
| Android           | CrossXcodeBuild | macOS, Linux        | `.apk`                             | APK and `custom` packagers                                           |
| HarmonyOS (`ohos`) | CrossXcodeBuild | macOS, Linux       | `.hap` (requires DevEco Studio)    | HAP and `custom` packagers                                           |

Xcode Builder is connected to the package process. The top-level `fastforge package` command has no dedicated flags for required arguments such as `project` and `scheme`, so the recommended entry point is a local workflow's `build-args`. String arguments can also be passed from the CLI through `--flutter-build-args`, for example `fastforge package --platform macos --targets zip --flutter-build-args project=MyApp.xcodeproj,scheme=MyApp,derived-data-path=build`; array arguments such as `extra-flags` require `build-args`.

For native projects, each target runs its own build, `flutter clean` is not run, the app name and version in the artifact name come from the app's `Info.plist`, and channel and flavor are not used.

## macOS

```yaml
- name: Package macOS app
  uses: fastforge/package
  with:
    platform: macos
    target: zip
    output: artifacts/
    build-args: '{"project":"MyApp.xcodeproj","scheme":"MyApp","configuration":"Release","derived-data-path":"build"}'
```

The builder runs `xcodebuild`, then searches the build products directory for an `.app`:

- With `derived-data-path`: `<derived-data-path>/Build/Products/<configuration>/`
- Without it: `<directory containing the project>/build/<configuration>/`

Set `derived-data-path` so the products location is explicit. The package format is validated only after the build, so an unsupported macOS format fails after `xcodebuild` finishes.

### Packaging configuration directory

The DMG, PKG, and ZIP packagers read `<packaging directory>/<format>/make_config.yaml` (and, for DMG, the assets next to it). For a native project the packaging directory is resolved in this order:

1. The `packaging-dir` build argument, relative to the working directory, when given.
2. `packaging/` next to the `.xcodeproj` (`<directory of project>/packaging/`), when that directory exists.
3. `macos/packaging/`, the Flutter layout and the fallback for native projects that already keep such a tree.

So a project at `apps/desktop/macos/MyApp.xcodeproj` with `apps/desktop/macos/packaging/pkg/make_config.yaml` can run Fastforge from `apps/desktop/macos` with `project=MyApp.xcodeproj`, and `dist/` and the derived data stay inside the project. The output names the `make_config.yaml` that was read, or says that none was found and defaults were used (for PKG, an unsigned package).

| Argument            | Required | Description                   |
| ------------------- | :------: | ----------------------------- |
| `project`           |   Yes    | Path to `.xcodeproj`          |
| `scheme`            |   Yes    | Xcode scheme                  |
| `configuration`     |    No    | Defaults to `Release`         |
| `derived-data-path` |    No    | Derived Data output directory |
| `packaging-dir`     |    No    | Directory holding `<format>/make_config.yaml`; see [above](#packaging-configuration-directory) |
| `product-name`      |    No    | `.app` name to match; otherwise any `.app` |
| `sdk`               |    No    | Passed to `xcodebuild -sdk`   |
| `xcconfig-override` |    No    | Additional xcconfig file      |
| `extra-flags`       |    No    | Array of additional arguments |

## iOS

```yaml
- name: Package iOS app
  uses: fastforge/package
  with:
    platform: ios
    target: ipa
    output: artifacts/
    build-args: '{"project":"ios/MyApp.xcodeproj","scheme":"MyApp","configuration":"Release","export-options-plist":"ios/ExportOptions.plist"}'
```

An iOS build has two stages:

1. Run `xcodebuild archive` to produce an `.xcarchive`.
2. Run `xcodebuild -exportArchive` to export the IPA. Fastforge uses the newest `.ipa` in `export-path`.

Only the `ipa` format is supported for native iOS projects; other formats are rejected before building.

| Argument               |   Required   | Description                                                       |
| ---------------------- | :----------: | ----------------------------------------------------------------- |
| `project`              |     Yes      | Path to `.xcodeproj`                                              |
| `scheme`               |     Yes      | Xcode scheme                                                      |
| `configuration`        |      No      | Defaults to `Release`                                             |
| `export-options-plist` | One required | Path to an ExportOptions plist; takes precedence                  |
| `export-method`        | One required | Generate temporary export configuration (automatic signing) when no plist is supplied |
| `archive-path`         |      No      | Defaults to `ios/build/Runner.xcarchive`                          |
| `export-path`          |      No      | Defaults to `ios/build/ipa`                                       |
| `derived-data-path`    |      No      | Derived Data output directory (archive step)                      |
| `xcconfig-override`    |      No      | Additional xcconfig file (archive step)                           |
| `extra-flags`          |      No      | Array of additional `xcodebuild archive` arguments                |

Provide at least one of `export-options-plist` and `export-method`. `project` is passed as `-project`; `.xcworkspace` is not supported.

## CrossXcodeBuild (Windows, Linux, Web, Android, HarmonyOS)

When a project without `pubspec.yaml` contains a `.xcodeproj` or `.xcworkspace` (or passes Xcode build arguments such as `project=*.xcodeproj`, `workspace=*.xcworkspace`, `crossswift-path`, or `xcodebuild-path`), Fastforge routes non-Apple platforms (`windows`, `linux`, `web`, `android`, `ohos`) to [CrossXcodeBuild](https://github.com/libnativeapi/crossswift).

```yaml
- name: Package Windows app from Xcode project
  uses: fastforge/package
  with:
    platform: windows
    target: zip
    output: artifacts/
    build-args: '{"project":"MyApp.xcodeproj","scheme":"MyApp","configuration":"Release","derived-data-path":"build"}'
```

### Tool discovery

Fastforge locates CrossXcodeBuild in this order:

1. `xcodebuild-path` (or `crossxcodebuild-path`) build argument.
2. `CROSSXCODEBUILD_PATH` (or `CROSSXCODEBUILD`) environment variable.
3. `crossxcodebuild` executable on `PATH`.
4. `xcodebuild` executable on `PATH` (directly on Windows and Linux; on macOS, only when `xcodebuild -version` reports `CrossSwift`, so Apple's `/usr/bin/xcodebuild` is never invoked).
5. `swift run --package-path <dir> xcodebuild` using `crossswift-path` or the `CROSS_SWIFTUI_PATH` environment variable.

### Staged output and metadata

Before building, Fastforge runs `-showBuildSettings -json` to read `BUILT_PRODUCTS_DIR`, `PRODUCT_NAME`, `EXECUTABLE_NAME`, `MARKETING_VERSION`, and `CURRENT_PROJECT_VERSION`, falling back to `<derived-data-path>/Build/Products/<configuration>/<product-name>/` (defaulting `derived-data-path` to `<project root>/build`).

- **Windows, Linux, Web**: the staged directory `<BUILT_PRODUCTS_DIR>/<ProductName>/` is passed directly to the platform's packagers.
- **Android**: Fastforge packages `<ProductName>.apk`. CrossXcodeBuild does not produce `.aab` bundles, so `aab` is rejected before building.
- **HarmonyOS (`ohos`)**: Fastforge packages `<ProductName>.hap` (requires DevEco Studio / `DEVECO_STUDIO_HOME` on the host). `app` bundles are not supported.

| Argument            | Required | Description                                                                            |
| ------------------- | :------: | -------------------------------------------------------------------------------------- |
| `project`           |    No    | Path to `.xcodeproj` (auto-discovered by CrossXcodeBuild when omitted)                 |
| `workspace`         |    No    | Path to `.xcworkspace` (mutually exclusive with `project`)                             |
| `scheme`            |    No    | Xcode scheme name                                                                      |
| `xcode-target`      |    No    | Xcode target name (passed as `-target`)                                                |
| `configuration`     |    No    | `Debug` or `Release` (defaults to `Release`)                                           |
| `derived-data-path` |    No    | Derived Data output directory (defaults to `<project root>/build`)                     |
| `destination`       |    No    | Passed as `-destination`; defaults from platform (`generic/platform=Windows\|Linux\|Wasm\|Android\|HarmonyOS`) |
| `sdk`               |    No    | Passed as `-sdk` (skips the default `-destination` when set)                           |
| `arch`              |    No    | Architecture hint (passed as `-arch`)                                                  |
| `xcconfig-override` |    No    | Additional `.xcconfig` file                                                            |
| `xcodebuild-path`   |    No    | Explicit path to the CrossXcodeBuild executable (alias: `crossxcodebuild-path`)        |
| `crossswift-path`   |    No    | Path to a `libnativeapi/crossswift` checkout (passed as `--crossswift-path`)           |
| `nativeapi-path`    |    No    | Path to `nativeapi/bindings/swift` (passed as `--nativeapi-path`)                      |
| `scratch-path`      |    No    | Intermediate SwiftPM build directory (passed as `--scratch-path`)                      |
| `console`           |    No    | Boolean; keeps the Windows console subsystem (`--console`)                             |
| `clean`             |    No    | Boolean; runs `clean` before `build`                                                   |
| `product-name`      |    No    | Overrides the staged product directory name when `-showBuildSettings` is unavailable   |
| `packaging-dir`     |    No    | Directory holding `<format>/make_config.yaml`                                          |
| `build-settings`    |    No    | Object of `KEY: VALUE` build setting overrides (for example `ANDROID_ABIS`, `CODE_SIGNING_ALLOWED`) |
| `extra-flags`       |    No    | Array of additional CrossXcodeBuild arguments                                          |
