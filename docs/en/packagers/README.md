# Packagers

English | [简体中文](../../zh-Hans/packagers/README.md)

This section organizes Fastforge packaging capabilities by platform. Each platform page describes connected build systems, output formats, and environment requirements.

## Platforms

| Platform                      | Formats                                                                                                        | `package` integration                   |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| [Android](android.md)         | [APK](android.md#apk), [AAB](android.md#aab)                                                                   | Gradle, Flutter Builder                 |
| [iOS](ios.md)                 | [IPA](ios.md#ipa)                                                                                              | Xcode (package action), Flutter Builder |
| [macOS](macos.md)             | [DMG](macos.md#dmg), [PKG](macos.md#pkg), [ZIP](macos.md#zip)                                                  | Xcode (package action), Flutter Builder |
| [Windows](windows.md)         | [EXE](windows.md#exe), [MSIX](windows.md#msix), [ZIP / direct](windows.md#zip-and-direct)                      | .NET, Flutter Builder                   |
| [Linux](linux.md)             | [AppImage](linux.md#appimage), [DEB](linux.md#deb), [RPM](linux.md#rpm), [Pacman](linux.md#pacman), [Flatpak](linux.md#flatpak), [Nix](linux.md#nix), [ZIP / direct](linux.md#zip-and-direct) | Flutter Builder |
| [Web](web.md)                 | [ZIP / direct](web.md)                                                                                         | Flutter Builder                         |
| [OpenHarmony](ohos.md)        | [HAP / APP](ohos.md)                                                                                           | Flutter Builder                         |
| Any                           | [custom](../packaging.md#custom-format)                                                                        | Flutter Builder, Xcode (macOS), .NET    |

`direct` copies the build output directory as-is instead of wrapping it in an archive or installer.

> [!IMPORTANT]
> Projects without `pubspec.yaml` can package only Android (Gradle), iOS and macOS (Xcode), and Windows (.NET). See the [builder overview](../builders/README.md) for builder selection rules.

## Choosing a Command

- To produce a distributable format, prefer `fastforge package`.
- To produce only a raw Flutter artifact, use `fastforge build`.
- For repeatable multi-step releases or publishing, use `fastforge workflow`.

Packaged artifacts are written to `dist/<version>/` by default. See [Packaging](../packaging.md) for the general process, options, artifact naming, and hooks.
