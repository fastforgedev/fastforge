# Flutter Builder

English | [简体中文](../../zh-Hans/builders/flutter.md)

Flutter Builder invokes the Flutter CLI from the project environment, builds for a platform, locates raw artifacts, and returns a normalized build result. It is used by `fastforge build` and by `fastforge package` in projects that contain `pubspec.yaml`.

## Project Requirements

- Run from the root of a project containing `pubspec.yaml`
- Install the Flutter SDK and make `flutter` available in `PATH` (or set `FLUTTER_ROOT`)
- Prepare the target platform SDK, signing configuration, and build tools

The builder reads `version` from `pubspec.yaml` and appends `--build-name` and `--build-number` to `flutter build`, unless those arguments are already provided.

## Platforms and Targets

| Platform  | Target           | Output         |
| --------- | ---------------- | -------------- |
| `android` | `apk`            | APK            |
| `android` | `aab`            | AAB            |
| `ios`     | `ipa` or omitted | IPA (`.app` with `no-codesign`) |
| `macos`   | May be omitted   | `.app`         |
| `windows` | May be omitted   | Windows bundle |
| `linux`   | May be omitted   | Linux bundle   |
| `web`     | May be omitted   | Web directory  |
| `ohos`    | `hap`, `app`     | HAP or APP     |

## Common Commands

```bash
fastforge build --platform android --target apk
fastforge build --platform web
fastforge build --platform macos
```

An iOS IPA requires export configuration:

```bash
fastforge build --platform ios --target ipa \
  --build-export-options-plist ios/ExportOptions.plist
```

You can also use `--build-export-method`. With `fastforge package`, which has no `--build-export-method` flag, use `--flutter-build-args export-method=app-store`.

To build without signing, pass `--flutter-build-args no-codesign`; no export configuration is needed. Flutter stops after the Xcode archive, and the build result is the `.app` inside it (the [IPA packager](../packagers/ios.md#ipa) wraps it into an unsigned IPA):

```bash
fastforge build --platform ios --target ipa --flutter-build-args no-codesign
```

Android APKs: the universal APK is the build result by default. With `--flutter-build-args split-per-abi`, the per-ABI split APKs are the result instead; see the [APK packager](../packagers/android.md#apk).

## Build Options

| Fastforge option                | Effect                                 | Available on         |
| ------------------------------- | -------------------------------------- | -------------------- |
| `--clean`                       | Run `flutter clean` before building    | `build`              |
| `--skip-clean`                  | Skip the default `flutter clean`       | `package`            |
| `--build-target`                | Use a custom entry point               | `build`, `package`   |
| `--build-flavor`                | Select a flavor                        | `build`, `package`   |
| `--build-target-platform`       | Select target architectures            | `build`, `package`   |
| `--build-export-options-plist`  | Provide iOS export configuration       | `build`, `package`   |
| `--build-export-method`         | Select an iOS export method            | `build`              |
| `--build-dart-define KEY=VALUE` | Compile-time variable; repeatable      | `build`, `package`   |
| `--build-obfuscate`             | Enable obfuscation                     | `build`              |
| `--build-split-debug-info`      | Set the debug-symbol output directory  | `build`              |
| `--build-tree-shake-icons`      | Enable icon tree shaking               | `build`              |
| `--build-profile`               | Use Profile mode                       | `build`              |
| `--flutter-build-args`          | Other build arguments, comma-separated | `build`, `package`   |

Within `--flutter-build-args`, entries without an equals sign are treated as boolean switches, while `key=value` entries become key-value arguments. Do not use this option when a value itself contains a comma; on `build`, a value also cannot contain `=`. On `package`, use it for options that have no dedicated flag, such as `profile` or `obfuscate,split-debug-info=build/symbols`.

## Artifact Locations

| Platform        | Default search location                                          |
| --------------- | ---------------------------------------------------------------- |
| Android APK     | `build/app/outputs/flutter-apk/` (universal APK, or split APKs with `split-per-abi`) |
| Android AAB     | `build/app/outputs/bundle/<mode>/` or `<flavor><Mode>/`          |
| iOS             | `build/ios/ipa/` (newest `.ipa`); with `no-codesign`, the `.app` in `build/ios/archive/*.xcarchive/Products/Applications/` |
| macOS           | `build/macos/Build/Products/<Mode>[-<flavor>]/`                  |
| Windows         | `build/windows/<arch>/runner/<Mode>/` (Flutter < 3.15: `build/windows/runner/<Mode>/`) |
| Linux           | `build/linux/<arch>/<mode>/bundle/`                              |
| Web             | `build/web/`                                                     |
| OpenHarmony HAP | `ohos/entry/build/<flavor>/outputs/<flavor>/` (`*-<flavor>-signed.hap`) |
| OpenHarmony APP | `ohos/build/outputs/<flavor>/` (`*-<flavor>-signed.app`)         |

For OpenHarmony, `<flavor>` defaults to `default`. On macOS, the `.app` named by `PRODUCT_NAME` in `macos/Runner/Configs/AppInfo.xcconfig` is preferred.

Fastforge reports a failure if the build command succeeds but no artifact is found in the expected directory. Windows, Linux, and Web builds report a directory rather than individual files; for them only the output directory must exist.

## Relationship to Packagers

`fastforge package` connects every platform above to its packagers; see [Packaging](../packaging.md#routing-and-support) for the full format matrix.

## Host Restrictions

- iOS and macOS builds run only on macOS.
- Windows builds run only on Windows.
- Linux builds run only on Linux.
- Android, Web, and OpenHarmony builders do not enforce a fixed host, but still require the relevant platform toolchains.

`fastforge build` fails on an unsupported host. `fastforge package` skips such a target with a warning and continues, exiting with status 0.

Flutter pubspec versions must use `major.minor.patch[-prerelease][+build]`. Invalid versions fail before invoking Flutter. Without `+build`, Fastforge does not supply a default `--build-number`; Flutter chooses its platform default. Explicit build arguments still win.
