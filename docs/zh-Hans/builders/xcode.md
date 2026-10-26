# Xcode Builder

[English](../../en/builders/xcode.md) | 简体中文

Xcode Builder 用于原生 iOS 和 macOS 项目（不含 `pubspec.yaml`），只能在安装了 Xcode 命令行工具的 macOS 上运行。

## 当前接入范围

| 平台  | 构建结果          | 后续打包                          |
| ----- | ----------------- | --------------------------------- |
| iOS   | `.xcarchive`、IPA | IPA 打包器                        |
| macOS | `.app`            | DMG、PKG、ZIP 和 `custom` 打包器 |

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
