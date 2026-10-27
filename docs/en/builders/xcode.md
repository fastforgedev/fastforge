# Xcode Builder

English | [简体中文](../../zh-Hans/builders/xcode.md)

Xcode Builder supports native iOS and macOS projects (projects without `pubspec.yaml`) and runs only on macOS with the Xcode command-line tools installed.

## Current Integration

| Platform | Build result      | Subsequent packaging        |
| -------- | ----------------- | --------------------------- |
| iOS      | `.xcarchive`, IPA | IPA packager                |
| macOS    | `.app`            | DMG, PKG, ZIP, and `custom` packagers |

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
