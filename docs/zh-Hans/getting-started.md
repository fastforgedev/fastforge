# 快速开始

[English](../en/getting-started.md) | 简体中文

本页从安装验证开始，介绍 Fastforge 的打包、发布和产物分析入口。平台细节放在对应的打包器文档中。

## 1. 检查环境

```bash
fastforge --version
fastforge --help
```

请在项目根目录运行命令，并提前安装目标平台所需的 SDK、构建工具和签名工具。

每个命令运行前都会查询 GitHub 上是否有新的 Fastforge 版本，并在 stderr 打印提示；可运行 `fastforge upgrade` 升级，或传入 `--no-version-check` 跳过检查（例如在 CI 中）。

## 2. 选择平台和格式

`package` 会执行平台构建并整理出可分发产物，可用范围取决于项目类型。

原生 Gradle Android 项目可以直接打包 APK 或 AAB：

```bash
fastforge package --platform android --target apk
fastforge package --platform android --target aab
```

Flutter 项目（含 `pubspec.yaml`）通过 Flutter Builder 构建，可以打包打包器支持的所有平台与格式组合，例如：

```bash
fastforge package --platform macos --target dmg
fastforge package --target apk   # target 无歧义时可省略 --platform
```

没有 `pubspec.yaml` 的项目只能打包 Android（Gradle Builder）、iOS 和 macOS（Xcode Builder）以及 Windows（.NET Builder）。Xcode 项目需要 `project` 和 `scheme`，.NET 项目需要 `project`，可通过 `--flutter-build-args project=...,scheme=...` 或工作流的 `build-args` 传入，详见 [Xcode Builder](builders/xcode.md) 和 [.NET Builder](builders/dotnet.md)。执行前可先查看[构建器总览](builders/README.md)和[打包器总览](packagers/README.md)确认覆盖范围。

## 3. 发布已有产物

```bash
fastforge publish \
  --path dist/app.apk \
  --target fir
```

发布参数和凭证见[发布器总览](publishers/README.md)。如果已经有产物，可以跳过打包，直接发布或分析。

## 4. 分析产物

`analyze` 会根据扩展名选择分析器，并输出 JSON：

```bash
fastforge analyze dist/app.apk
```

写入文件：

```bash
fastforge analyze dist/app.apk \
  --output analysis.json
```

当前支持 `.apk`、`.aab`、`.ipa`、`.dmg` 和 macOS `.app` bundle。DMG 与 `.app` 分析只支持 macOS。

## 下一步

- 单独执行构建：[构建](building.md)
- 制作安装包：[打包](packaging.md)
- 上传现有产物：[发布](publishing.md)
- 运行自动化任务：[本地工作流](workflows.md)
- 分析应用产物：[应用包分析](tools/analyze.md)
- 管理商店元数据：[商店管理](stores/README.md)
