# iOS

English | [简体中文](../../zh-Hans/packagers/ios.md)

Fastforge builds and prepares iOS application artifacts, producing [IPA](#ipa) files. Building iOS artifacts requires macOS and Xcode.

## Current Status

| Build system    | Project type            | `package` status                                   |
| --------------- | ----------------------- | -------------------------------------------------- |
| Xcode           | No `pubspec.yaml`       | IPA supported; package action recommended          |
| Flutter Builder | Contains `pubspec.yaml` | IPA supported through the CLI and package action   |

Both paths require export configuration, except for an unsigned Flutter IPA. Xcode Builder also requires `project` and `scheme`, which are best supplied through a workflow package action's `build-args`. The packaged IPA is copied to `dist/<version>/`.

## IPA

IPA is the archive distribution format for iOS applications.

For a Flutter project, provide an export options plist:

```bash
fastforge package --targets ipa \
  --build-export-options-plist ios/ExportOptions.plist
```

Or use an export method through `--flutter-build-args export-method=app-store`. See [Flutter Builder](../builders/flutter.md) for all options.

To package without signing, for example to sign the IPA later or for a CI check, use `--flutter-build-args no-codesign`. No export configuration is needed: Flutter stops after the Xcode archive, and Fastforge wraps its `.app` as `Payload/<name>.app` in the IPA.

Package an Xcode project through a workflow:

```yaml
- name: Package IPA
  uses: fastforge/package
  with:
    platform: ios
    target: ipa
    output: artifacts/
    build-args: '{"project":"ios/MyApp.xcodeproj","scheme":"MyApp","export-options-plist":"ios/ExportOptions.plist"}'
```

See [Xcode Builder](../builders/xcode.md#ios) for all options.

To generate only the raw IPA for a Flutter project:

```bash
fastforge build \
  --platform ios \
  --target ipa \
  --build-export-method app-store
```

### Publish to the App Store

```bash
fastforge publish --path dist/<version>/<artifact>.ipa --targets appstore
```

See the [App Store publisher](../publishers/appstore.md) and [App Store Connect](../stores/appstore.md) for credentials, uploads, and review workflows.

Return to the [packager overview](README.md).
