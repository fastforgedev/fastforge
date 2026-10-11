# Flutter Builder

[English](../../en/builders/flutter.md) | 简体中文

Flutter Builder 调用项目环境中的 Flutter CLI，负责执行平台构建、定位原始产物并返回统一的构建结果。`fastforge build` 以及含 `pubspec.yaml` 项目中的 `fastforge package` 都使用该构建器。

## 项目要求

- 在包含 `pubspec.yaml` 的项目根目录执行
- Flutter SDK 已安装，且 `flutter` 位于 `PATH`（或设置 `FLUTTER_ROOT`）
- 已准备目标平台的 SDK、签名配置和构建工具

构建器会读取 `pubspec.yaml` 中的 `version`，并为 `flutter build` 追加 `--build-name` 和 `--build-number` 参数（已显式提供时除外）。

## 平台与 target

| 平台      | target       | 输出           |
| --------- | ------------ | -------------- |
| `android` | `apk`        | APK            |
| `android` | `aab`        | AAB            |
| `ios`     | `ipa` 或省略 | IPA            |
| `macos`   | 可省略       | `.app`         |
| `windows` | 可省略       | Windows bundle |
| `linux`   | 可省略       | Linux bundle   |
| `web`     | 可省略       | Web 目录       |
| `ohos`    | `hap`、`app` | HAP 或 APP     |

## 常用命令

```bash
fastforge build --platform android --target apk
fastforge build --platform web
fastforge build --platform macos
```

iOS IPA 必须提供导出配置：

```bash
fastforge build --platform ios --target ipa \
  --build-export-options-plist ios/ExportOptions.plist
```

也可以使用 `--build-export-method`。`fastforge package` 没有 `--build-export-method` 参数，请改用 `--flutter-build-args export-method=app-store`。

## 构建参数

| Fastforge 参数                  | 作用                           | 适用命令           |
| ------------------------------- | ------------------------------ | ------------------ |
| `--clean`                       | 构建前执行 `flutter clean`     | `build`            |
| `--skip-clean`                  | 跳过默认的 `flutter clean`     | `package`          |
| `--build-target`                | 自定义入口文件                 | `build`、`package` |
| `--build-flavor`                | 指定 flavor                    | `build`、`package` |
| `--build-target-platform`       | 指定目标架构                   | `build`、`package` |
| `--build-export-options-plist`  | iOS 导出配置                   | `build`、`package` |
| `--build-export-method`         | iOS 导出方式                   | `build`            |
| `--build-dart-define KEY=VALUE` | 编译变量；可重复               | `build`、`package` |
| `--build-obfuscate`             | 开启 obfuscate                 | `build`            |
| `--build-split-debug-info`      | 调试符号输出目录               | `build`            |
| `--build-tree-shake-icons`      | 开启 icon tree shaking         | `build`            |
| `--build-profile`               | 使用 Profile 模式              | `build`            |
| `--flutter-build-args`          | 逗号分隔的其他构建参数         | `build`、`package` |

`--flutter-build-args` 中，无等号条目按布尔开关处理，`key=value` 按键值参数处理。值本身包含逗号时不应使用该入口。在 `package` 中，没有专用参数的选项可以通过它传入，例如 `profile` 或 `obfuscate,split-debug-info=build/symbols`。

## 产物定位

| 平台            | 默认查找位置                                                         |
| --------------- | -------------------------------------------------------------------- |
| Android APK     | `build/app/outputs/flutter-apk/`                                     |
| Android AAB     | `build/app/outputs/bundle/<mode>/` 或 `<flavor><Mode>/`              |
| iOS             | `build/ios/ipa/`（最新的 `.ipa`）                                    |
| macOS           | `build/macos/Build/Products/<Mode>[-<flavor>]/`                      |
| Windows         | `build/windows/<arch>/runner/<Mode>/`（Flutter 3.15 以前为 `build/windows/runner/<Mode>/`） |
| Linux           | `build/linux/<arch>/<mode>/bundle/`                                  |
| Web             | `build/web/`                                                         |
| OpenHarmony HAP | `ohos/entry/build/<flavor>/outputs/<flavor>/`（`*-<flavor>-signed.hap`） |
| OpenHarmony APP | `ohos/build/outputs/<flavor>/`（`*-<flavor>-signed.app`）            |

OpenHarmony 的 `<flavor>` 默认为 `default`。macOS 会优先匹配 `macos/Runner/Configs/AppInfo.xcconfig` 中 `PRODUCT_NAME` 对应的 `.app`。

构建命令成功但没有在预期目录找到产物时，Fastforge 会返回失败。Windows、Linux 和 Web 构建输出的是目录而不是单个文件，只要求输出目录存在。

## 与打包器的关系

`fastforge package` 已为上述所有平台接通对应打包器，完整格式矩阵见[打包](../packaging.md#路由与支持范围)。

## 宿主限制

- iOS 和 macOS 构建只能在 macOS 上执行。
- Windows 构建只能在 Windows 上执行。
- Linux 构建只能在 Linux 上执行。
- Android、Web 和 OpenHarmony 构建器不设置固定宿主限制，但仍依赖对应平台工具链。

`fastforge build` 在不支持的宿主上会失败；`fastforge package` 则输出警告并跳过该 target，继续执行并以状态码 0 退出。

Flutter pubspec 版本必须使用 `major.minor.patch[-prerelease][+build]`。无效版本会在调用 Flutter 前报错。版本没有 `+build` 时，Fastforge 不再传入默认 `--build-number`，由 Flutter 使用平台默认值；显式构建参数仍优先。
