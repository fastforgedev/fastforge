# 打包器

[English](../../en/packagers/README.md) | 简体中文

本节按平台组织 Fastforge 的打包能力。每个平台页面都会标注接入的构建系统、输出格式和环境要求。

## 平台

| 平台                   | 格式                                                                                                                                     | `package` 接入范围                       |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| [Android](android.md)  | [APK](android.md#apk)、[AAB](android.md#aab)                                                                                             | Gradle、Flutter Builder                  |
| [iOS](ios.md)          | [IPA](ios.md#ipa)                                                                                                                        | Xcode（package action）、Flutter Builder |
| [macOS](macos.md)      | [DMG](macos.md#dmg)、[PKG](macos.md#pkg)、[ZIP](macos.md#zip)                                                                            | Xcode（package action）、Flutter Builder |
| [Windows](windows.md)  | [EXE](windows.md#exe)、[MSIX](windows.md#msix)、[ZIP / direct](windows.md#zip-和-direct)                                                 | .NET、Flutter Builder                    |
| [Linux](linux.md)      | [AppImage](linux.md#appimage)、[DEB](linux.md#deb)、[RPM](linux.md#rpm)、[Pacman](linux.md#pacman)、[Flatpak](linux.md#flatpak)、[Nix](linux.md#nix)、[ZIP / direct](linux.md#zip-和-direct) | Flutter Builder                        |
| [Web](web.md)          | [ZIP / direct](web.md)                                                                                                                   | Flutter Builder                          |
| [OpenHarmony](ohos.md) | [HAP / APP](ohos.md)                                                                                                                     | Flutter Builder                          |
| 任意平台               | [custom](../packaging.md#自定义格式)                                                                                                     | Flutter Builder、Xcode（macOS）、.NET    |

`direct` 直接复制构建输出目录，不打成归档或安装包。

> [!IMPORTANT]
> 不含 `pubspec.yaml` 的项目只能打包 Android（Gradle）、iOS 和 macOS（Xcode）以及 Windows（.NET）。构建器的选择规则见[构建器总览](../builders/README.md)。

## 命令选择

- 需要可分发格式：优先使用 `fastforge package`。
- 只需要 Flutter 原始产物：使用 `fastforge build`。
- 需要可重复的多步骤发布或上传：使用 `fastforge workflow`。

打包产物默认写入 `dist/<version>/`。通用流程、参数、产物命名与钩子见[打包](../packaging.md)。
