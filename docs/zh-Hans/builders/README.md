# 构建器

[English](../../en/builders/README.md) | 简体中文

构建器负责选择项目构建工具、执行构建命令并定位生成的产物。打包器只消费构建结果，两者的职责不同。

## 当前构建器

| 构建器               | 平台或目标                                            | CLI 接入状态                              | 文档                                      |
| -------------------- | ----------------------------------------------------- | ----------------------------------------- | ----------------------------------------- |
| Gradle Android       | Android APK、AAB                                      | 已接入 `package` 和 package action        | [Gradle](gradle.md)                       |
| Gradle Multiplatform | Android、桌面、iOS framework                          | 暂未接入顶层 CLI                          | [Gradle](gradle.md#multiplatform-builder) |
| Xcode                | iOS IPA、macOS `.app`                                 | 已接入 package action 和 `package`        | [Xcode](xcode.md)                         |
| .NET                 | Windows `dotnet publish` 目录                         | 已接入 package action 和 `package`        | [.NET](dotnet.md)                         |
| Flutter              | Android、iOS、macOS、Windows、Linux、Web、OpenHarmony | 已接入 `build`、`package` 和 package action | [Flutter Builder](flutter.md)           |
| Custom               | 用户自定义命令和产物规则                              | 暂未接入顶层 CLI                          | [Custom Builder](custom.md)               |

“已实现”与“已接入 CLI”不是同一状态。没有顶层入口的构建器不能通过 Fastforge 命令选择。

## 当前路由方式

`fastforge package` 和 `fastforge/package` action 按项目根目录是否存在 `pubspec.yaml` 以及平台（通过 `--platform` 指定或根据 target 推断）选择构建路径：

1. 不存在 `pubspec.yaml` 且平台为 `macos` 或 `ios`：选择 Xcode Builder。
2. 不存在 `pubspec.yaml` 且平台为 `android`：选择 Gradle Builder。
3. 不存在 `pubspec.yaml` 且平台为 `windows`：选择 .NET Builder。
4. 不存在 `pubspec.yaml` 且为其他平台：报错，这些平台必须是 Flutter 项目。
5. 存在 `pubspec.yaml`：选择 Flutter Builder。

`fastforge build` 不做路由，始终使用 Flutter Builder。

当前判断规则较简单。执行命令前应确认工作目录就是项目根目录，避免选择到错误的构建器。

## 如何选择入口

- Flutter 项目：使用 `fastforge package` 为任意平台生成可分发格式，或使用 `fastforge build` 生成原始产物。
- 原生 Android Gradle 项目：使用 `fastforge package` 或工作流 package action。
- 原生 iOS / macOS Xcode 项目：使用工作流 package action，以 `build-args` 提供工程参数。
- 原生 Windows .NET 项目：使用工作流 package action，或 `fastforge package` 配合 `--flutter-build-args project=...`，见 [.NET Builder](dotnet.md)。
- 需要尚未接入的构建器：当前应继续使用项目自己的构建命令，不要假设 Fastforge CLI 已支持。

构建命令和结果结构见[构建](../building.md)。
