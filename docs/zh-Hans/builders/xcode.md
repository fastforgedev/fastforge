# Xcode Builder

[English](../../en/builders/xcode.md) | 简体中文

Xcode Builder 用于原生 Xcode 项目（不含 `pubspec.yaml`）：

- **iOS 和 macOS** 在安装了 Xcode 命令行工具的 macOS 上调用 Apple `xcodebuild`。
- **Windows、Linux、Web（WebAssembly）、Android 和 HarmonyOS（`ohos`）** 通过 [CrossXcodeBuild](https://github.com/libnativeapi/crossswift)（`libnativeapi/crossswift`）编译未修改的 `.xcodeproj` / `.xcworkspace` 工程。

## 当前接入范围

| 平台               | 构建工具        | 允许的构建宿主 | 构建结果                                   | 后续打包                                                                 |
| ------------------ | --------------- | -------------- | ------------------------------------------ | ------------------------------------------------------------------------ |
| iOS                | Apple xcodebuild | macOS          | `.xcarchive`、IPA                          | IPA 打包器                                                               |
| macOS              | Apple xcodebuild | macOS          | `.app`                                     | DMG、PKG、ZIP、`tar.gz` 和 `custom` 打包器                               |
| Windows            | CrossXcodeBuild | Windows        | 应用输出目录（`.exe`、DLL）                | EXE、MSIX、ZIP、`tar.gz`、`direct` 和 `custom` 打包器                    |
| Linux              | CrossXcodeBuild | Linux          | 应用输出目录（`lib/*.so`）                 | DEB、RPM、Pacman、AppImage、Flatpak、Nix、ZIP、`tar.gz`、`direct`、`custom` |
| Web（WebAssembly） | CrossXcodeBuild | macOS、Linux   | Web 输出目录（`index.html`、`.wasm`）      | ZIP、`tar.gz`、`direct` 和 `custom` 打包器                               |
| Android            | CrossXcodeBuild | macOS、Linux   | `.apk`                                     | APK 和 `custom` 打包器                                                   |
| HarmonyOS（`ohos`） | CrossXcodeBuild | macOS、Linux   | `.hap`（需要 DevEco Studio）               | HAP 和 `custom` 打包器                                                   |

Xcode Builder 已经接入 package 流程。顶层 `fastforge package` 没有 `project`、`scheme` 等必要参数的专用选项，推荐通过本地工作流的 `build-args` 使用。字符串参数也可以通过 CLI 的 `--flutter-build-args` 传入，例如 `fastforge package --platform macos --targets zip --flutter-build-args project=MyApp.xcodeproj,scheme=MyApp,derived-data-path=build`；`extra-flags` 等数组参数需要使用 `build-args`。

对于原生项目，每个 target 单独构建，不执行 `flutter clean`，产物名中的应用名和版本取自应用的 `Info.plist`，不使用 channel 和 flavor。

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

构建器执行 `xcodebuild`，然后从构建产品目录中查找 `.app`：

- 提供 `derived-data-path` 时：`<derived-data-path>/Build/Products/<configuration>/`
- 未提供时：`<project 所在目录>/build/<configuration>/`

建议设置 `derived-data-path`，明确产品目录位置。打包格式在构建完成后才校验，因此不支持的 macOS 格式会在 `xcodebuild` 结束后才失败。

### 打包配置目录

DMG、PKG 和 ZIP 打包器读取 `<打包配置目录>/<format>/make_config.yaml`（DMG 还会使用旁边的资源文件）。原生项目的打包配置目录按以下顺序解析：

1. 提供了 `packaging-dir` 构建参数时使用该目录，相对于工作目录。
2. `.xcodeproj` 旁边的 `packaging/`（`<project 所在目录>/packaging/`），该目录存在时使用。
3. `macos/packaging/`，即 Flutter 布局，也是已经保留这一目录结构的原生项目的回退路径。

因此位于 `apps/desktop/macos/MyApp.xcodeproj` 的项目，配合 `apps/desktop/macos/packaging/pkg/make_config.yaml`，可以在 `apps/desktop/macos` 目录下以 `project=MyApp.xcodeproj` 运行 Fastforge，`dist/` 和 Derived Data 也留在项目内。输出会写明读取了哪个 `make_config.yaml`，或者说明未找到配置、使用了默认值（对 PKG 来说即未签名的安装包）。

| 参数                | 必填 | 说明                                  |
| ------------------- | :--: | ------------------------------------- |
| `project`           |  是  | `.xcodeproj` 路径                     |
| `scheme`            |  是  | Xcode scheme                          |
| `configuration`     |  否  | 默认 `Release`                        |
| `derived-data-path` |  否  | Derived Data 输出目录                 |
| `packaging-dir`     |  否  | 存放 `<format>/make_config.yaml` 的目录，见[上文](#打包配置目录) |
| `product-name`      |  否  | 要匹配的 `.app` 名称；未提供时匹配任意 `.app` |
| `sdk`               |  否  | 传给 `xcodebuild -sdk`                |
| `xcconfig-override` |  否  | 额外 xcconfig 文件                    |
| `extra-flags`       |  否  | 额外参数数组                          |

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

iOS 构建分为两个步骤：

1. 使用 `xcodebuild archive` 生成 `.xcarchive`。
2. 使用 `xcodebuild -exportArchive` 导出 IPA。Fastforge 使用 `export-path` 中最新的 `.ipa`。

原生 iOS 项目只支持 `ipa` 格式，其他格式会在构建前被拒绝。

| 参数                   |  必填  | 说明                                            |
| ---------------------- | :----: | ----------------------------------------------- |
| `project`              |   是   | `.xcodeproj` 路径                               |
| `scheme`               |   是   | Xcode scheme                                    |
| `configuration`        |   否   | 默认 `Release`                                  |
| `export-options-plist` | 二选一 | ExportOptions plist 路径；优先使用              |
| `export-method`        | 二选一 | 未提供 plist 时生成临时导出配置（自动签名）     |
| `archive-path`         |   否   | 默认 `ios/build/Runner.xcarchive`               |
| `export-path`          |   否   | 默认 `ios/build/ipa`                            |
| `derived-data-path`    |   否   | Derived Data 输出目录（archive 步骤）           |
| `xcconfig-override`    |   否   | 额外 xcconfig 文件（archive 步骤）              |
| `extra-flags`          |   否   | 额外 `xcodebuild archive` 参数数组              |

`export-options-plist` 和 `export-method` 至少提供一个。`project` 以 `-project` 传入，不支持 `.xcworkspace`。

## CrossXcodeBuild（Windows、Linux、Web、Android、HarmonyOS）

当不含 `pubspec.yaml` 的项目包含 `.xcodeproj` / `.xcworkspace`（或传入 `project=*.xcodeproj`、`workspace=*.xcworkspace`、`crossswift-path`、`xcodebuild-path` 等参数）时，Fastforge 会将非 Apple 平台（`windows`、`linux`、`web`、`android`、`ohos`）路由到 [CrossXcodeBuild](https://github.com/libnativeapi/crossswift)。

```yaml
- name: 使用 CrossXcodeBuild 打包 Windows 应用
  uses: fastforge/package
  with:
    platform: windows
    target: zip
    output: artifacts/
    build-args: '{"project":"MyApp.xcodeproj","scheme":"MyApp","configuration":"Release","derived-data-path":"build"}'
```

### 工具查找顺序

Fastforge 按以下顺序查找 CrossXcodeBuild：

1. `xcodebuild-path`（或别名 `crossxcodebuild-path`）构建参数。
2. `CROSSXCODEBUILD_PATH`（或 `CROSSXCODEBUILD`）环境变量。
3. `PATH` 中的 `crossxcodebuild` 可执行文件。
4. `PATH` 中的 `xcodebuild` 可执行文件（Windows 与 Linux 直接使用；macOS 上仅在 `xcodebuild -version` 输出包含 `CrossSwift` 时使用，避免误调用 Apple `/usr/bin/xcodebuild`）。
5. 使用 `crossswift-path` 参数或 `CROSS_SWIFTUI_PATH` 环境变量执行 `swift run --package-path <目录> xcodebuild`。

### 输出目录与元数据

构建前，Fastforge 会调用 `-showBuildSettings -json` 读取 `BUILT_PRODUCTS_DIR`、`PRODUCT_NAME`、`EXECUTABLE_NAME`、`MARKETING_VERSION` 和 `CURRENT_PROJECT_VERSION`；不可用时回退到 `<derived-data-path>/Build/Products/<configuration>/<product-name>/`（`derived-data-path` 默认为 `<工程根目录>/build`）。

- **Windows、Linux、Web**：直接将 `<BUILT_PRODUCTS_DIR>/<ProductName>/` 交给对应平台的打包器。
- **Android**：打包 `<ProductName>.apk`。CrossXcodeBuild 当前不产出 `.aab`，请求 `aab` 会在构建前被拒绝。
- **HarmonyOS（`ohos`）**：打包 `<ProductName>.hap`（需要本机安装 DevEco Studio / 设置 `DEVECO_STUDIO_HOME`），不支持 `app` 格式。

| 参数                | 必填 | 说明                                                                             |
| ------------------- | :--: | -------------------------------------------------------------------------------- |
| `project`           |  否  | `.xcodeproj` 路径（未提供时由 CrossXcodeBuild 自动发现）                         |
| `workspace`         |  否  | `.xcworkspace` 路径（与 `project` 互斥）                                         |
| `scheme`            |  否  | Xcode scheme 名称                                                                |
| `xcode-target`      |  否  | Xcode target 名称（以 `-target` 传入）                                           |
| `configuration`     |  否  | `Debug` 或 `Release`（默认 `Release`）                                           |
| `derived-data-path` |  否  | Derived Data 输出目录（默认 `<工程根目录>/build`）                               |
| `destination`       |  否  | 传给 `-destination`；未设置时按平台自动选择（`generic/platform=Windows\|Linux\|Wasm\|Android\|HarmonyOS`） |
| `sdk`               |  否  | 传给 `-sdk`（设置后不再自动附加默认 `-destination`）                             |
| `arch`              |  否  | 架构提示（以 `-arch` 传入）                                                      |
| `xcconfig-override` |  否  | 额外 `.xcconfig` 文件                                                            |
| `xcodebuild-path`   |  否  | 显式指定 CrossXcodeBuild 可执行文件路径（别名：`crossxcodebuild-path`）          |
| `crossswift-path`   |  否  | `libnativeapi/crossswift` 源码目录路径（以 `--crossswift-path` 传入）            |
| `nativeapi-path`    |  否  | `nativeapi/bindings/swift` 目录路径（以 `--nativeapi-path` 传入）                |
| `scratch-path`      |  否  | SwiftPM 中间构建目录（以 `--scratch-path` 传入）                                 |
| `console`           |  否  | 布尔值；保留 Windows 控制台子系统（`--console`）                                 |
| `clean`             |  否  | 布尔值；构建前执行 `clean`                                                       |
| `product-name`      |  否  | `-showBuildSettings` 不可用时覆盖产品目录名称                                    |
| `packaging-dir`     |  否  | 存放 `<format>/make_config.yaml` 的目录                                          |
| `build-settings`    |  否  | `KEY: VALUE` 构建设置覆盖对象（如 `ANDROID_ABIS`、`CODE_SIGNING_ALLOWED`）       |
| `extra-flags`       |  否  | 额外 CrossXcodeBuild 参数数组                                                    |
