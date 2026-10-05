# Fastforge CLI 文档

[English](../en/README.md) | 简体中文

本目录记录 Fastforge 当前 CLI 的实际用法，内容以仓库中的命令实现、核心模块和安装脚本为准。

旧版文档继续保留在 [`apps/docs`](../../apps/docs/)；本目录不会替换或修改旧版文档。

> [!IMPORTANT]
> 当前 CLI 仍在持续完善。文档会区分“已经实现”和“暂未实现”的能力。若命令帮助与文档不一致，以 `fastforge <command> --help` 为准。

## 文档导航

### 指南

- [安装](installation.md)：安装原生二进制、从源码安装和环境要求
- [快速开始](getting-started.md)：验证安装、打包项目和分析产物
- [构建](building.md)：构建命令、构建结果和构建器边界
- [打包](packaging.md)：打包流程、构建阶段和生命周期钩子
- [发布](publishing.md)：发布目标、参数和凭证
- [本地工作流](workflows.md)：发现、校验和运行 `.fastforge/workflows`
- [远程主机](remote-hosts.md)：通过 SSH 在其他机器上打包和运行
- [CLI 参考](cli.md)：顶层命令及参数速查

### 构建器

- [构建器总览](builders/README.md)
- [Gradle](builders/gradle.md) · [Xcode](builders/xcode.md)
- [Flutter Builder](builders/flutter.md) · [Custom Builder](builders/custom.md)

### 打包器

- [打包器总览](packagers/README.md)
- [Android](packagers/android.md)：[APK](packagers/android.md#apk) · [AAB](packagers/android.md#aab)
- [iOS](packagers/ios.md)：[IPA](packagers/ios.md#ipa)
- [macOS](packagers/macos.md)：[DMG](packagers/macos.md#dmg) · [PKG](packagers/macos.md#pkg) · [ZIP](packagers/macos.md#zip)
- [Windows](packagers/windows.md) · [Linux](packagers/linux.md) · [Web](packagers/web.md) · [OpenHarmony](packagers/ohos.md)

### 发布目标

- [发布器总览](publishers/README.md)
- [对象存储](publishers/s3.md) · [fir.im](publishers/fir.md) · [蒲公英](publishers/pgyer.md) · [Firebase](publishers/firebase.md)
- [GitHub](publishers/github.md) · [App Store](publishers/appstore.md) · [Google Play](publishers/playstore.md) · [AppGallery](publishers/appgallery.md)
- [Vercel](publishers/vercel.md) · [Custom](publishers/custom.md)

### 商店与工具

- [商店管理](stores/README.md)：App Store Connect、AppGallery Connect、Google Play 和 catalog 同步
- [应用包分析](tools/analyze.md)：APK、AAB、IPA、DMG 与 `.app`

## 当前能力概览

| 能力                     | 状态           | 入口                                |
| ------------------------ | -------------- | ----------------------------------- |
| 应用包分析               | 已实现         | [Analyze](tools/analyze.md)         |
| 多平台打包               | 已实现         | [打包器总览](packagers/README.md)   |
| 构建器                   | 部分实现       | [构建器总览](builders/README.md)    |
| 产物发布                 | 已实现多个目标 | [发布器总览](publishers/README.md)  |
| 本地工作流               | 已实现         | [本地工作流](workflows.md)          |
| App Store Connect        | 已实现         | [App Store](stores/appstore.md)     |
| AppGallery Connect       | 已实现         | [AppGallery](stores/appgallery.md)  |
| Google Play Console      | 已实现         | [Google Play](stores/googleplay.md) |
| 多商店 catalog 同步      | 已实现         | [Catalog](stores/catalog.md)        |
| Studio（本地 Web 界面）  | 已实现         | [`fastforge studio`](cli.md#studio) |
| 自动升级                 | 已实现         | [`fastforge upgrade`](installation.md#升级) |
| 在线版本检查             | 已实现         | 每次运行命令前，或 `fastforge version-check` |

## 获取当前版本信息

```bash
fastforge --version
fastforge --help
fastforge version-check --current-only
```
