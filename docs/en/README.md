# Fastforge CLI Documentation

English | [简体中文](../zh-Hans/README.md)

This directory documents the actual behavior of the current Fastforge CLI. Its contents follow the command implementations, core modules, and installation scripts in this repository.

The legacy documentation remains in [`apps/docs`](../../apps/docs/). This directory does not replace or modify the legacy documentation.

> [!IMPORTANT]
> The current CLI is still under active development. The documentation distinguishes between implemented and not-yet-implemented features. If the command help differs from the documentation, follow `fastforge <command> --help`.

## Documentation

### Guides

- [Installation](installation.md): install the native binary, build from source, and review environment requirements
- [Getting Started](getting-started.md): verify the installation, package a project, and analyze artifacts
- [Building](building.md): build commands, build results, and builder boundaries
- [Packaging](packaging.md): the packaging process, build stage, and lifecycle hooks
- [Publishing](publishing.md): publishing targets, parameters, and credentials
- [Local Workflows](workflows.md): discover, validate, and run `.fastforge/workflows`
- [Remote Hosts](remote-hosts.md): package and run on another machine over SSH
- [CLI Reference](cli.md): top-level commands and options

### Builders

- [Builder overview](builders/README.md)
- [Gradle](builders/gradle.md) · [Xcode](builders/xcode.md)
- [Flutter Builder](builders/flutter.md) · [Custom Builder](builders/custom.md)

### Packagers

- [Packager overview](packagers/README.md)
- [Android](packagers/android.md): [APK](packagers/android.md#apk) · [AAB](packagers/android.md#aab)
- [iOS](packagers/ios.md): [IPA](packagers/ios.md#ipa)
- [macOS](packagers/macos.md): [DMG](packagers/macos.md#dmg) · [PKG](packagers/macos.md#pkg) · [ZIP](packagers/macos.md#zip)
- [Windows](packagers/windows.md) · [Linux](packagers/linux.md) · [Web](packagers/web.md) · [OpenHarmony](packagers/ohos.md)

### Publishing Targets

- [Publisher overview](publishers/README.md)
- [Object storage](publishers/s3.md) · [fir.im](publishers/fir.md) · [PGYER](publishers/pgyer.md) · [Firebase](publishers/firebase.md)
- [GitHub](publishers/github.md) · [App Store](publishers/appstore.md) · [Google Play](publishers/playstore.md) · [AppGallery](publishers/appgallery.md)
- [Vercel](publishers/vercel.md) · [Custom](publishers/custom.md)

### Stores and Tools

- [Store management](stores/README.md): App Store Connect, AppGallery Connect, Google Play, and catalog synchronization
- [App package analysis](tools/analyze.md): APK, AAB, IPA, DMG, and `.app`

## Current Capabilities

| Capability                    | Status                       | Entry point                                             |
| ----------------------------- | ---------------------------- | ------------------------------------------------------- |
| App package analysis          | Implemented                  | [Analyze](tools/analyze.md)                             |
| Multi-platform packaging      | Implemented                  | [Packager overview](packagers/README.md)                |
| Builders                      | Partially implemented        | [Builder overview](builders/README.md)                  |
| Artifact publishing           | Multiple targets implemented | [Publisher overview](publishers/README.md)              |
| Local workflows               | Implemented                  | [Local workflows](workflows.md)                         |
| App Store Connect             | Implemented                  | [App Store](stores/appstore.md)                         |
| AppGallery Connect            | Implemented                  | [AppGallery](stores/appgallery.md)                      |
| Google Play Console           | Implemented                  | [Google Play](stores/googleplay.md)                     |
| Multi-store catalog sync      | Implemented                  | [Catalog](stores/catalog.md)                            |
| Studio (local web UI)         | Implemented                  | [`fastforge studio`](cli.md#studio)                     |
| Self-upgrade                  | Implemented                  | [`fastforge upgrade`](installation.md#upgrade)          |
| Online version checks         | Implemented                  | Before every command, or `fastforge version-check`      |

## Get Current Version Information

```bash
fastforge --version
fastforge --help
fastforge version-check --current-only
```
