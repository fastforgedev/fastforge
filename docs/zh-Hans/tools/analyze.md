# 应用包分析

[English](../../en/tools/analyze.md) | 简体中文

`fastforge analyze` 从应用产物读取身份信息——名称、标识符、版本和构建号——并对下列
格式进一步给出产物的组成：它构建在什么技术之上、带了哪些依赖库、体积由什么构成，
以及签名状态。

命令接受任意多个产物，也可以传入目录并扫描其中的包。结果以 JSON 输出，或渲染成一份
独立的 HTML 报告。

## 支持格式

| 格式          | 平台限制       | 依赖                                                              |
| ------------- | -------------- | ----------------------------------------------------------------- |
| APK           | 无固定宿主限制 | `ANDROID_HOME` 中的 `aapt2`；可选 `apksigner`                     |
| AAB           | 无固定宿主限制 | `aapt2`，或 `BUNDLETOOL`                                          |
| IPA           | 无固定宿主限制 | 无外部工具                                                        |
| DMG           | 仅 macOS       | `hdiutil`、`diskutil`；可选 `codesign`、`spctl`、`xcrun stapler`  |
| `.app` bundle | 仅 macOS       | 本地 `Info.plist`；可选 `codesign`、`spctl`、`xcrun stapler`      |

## 输出到终端

```bash
fastforge analyze dist/app-release.apk
```

所有格式都会在顶层给出相同的身份信息，以及文件自身的事实：

```json
{
  "platform": "android",
  "format": "apk",
  "identifier": "com.example.app",
  "name": "Example",
  "version": "1.0.0",
  "buildNumber": 1,
  "fileName": "app-release.apk",
  "sizeBytes": 55605086,
  "sha256": "3d60a82610b81ce760bc999f5f36917b41256a43184f401a747923c0c01c8a5e"
}
```

## 输出到文件

```bash
fastforge analyze dist/app-release.apk \
  --output analysis.json
```

## 一次分析多个产物

每个参数要么是产物，要么是待扫描的目录。目录会被递归遍历，跳过隐藏项且不跟随符号
链接；`.app` bundle 被当作产物而不是继续深入的目录。

```bash
fastforge analyze dist
fastforge analyze dist/android dist/macos build/ios/ipa
```

分析多个产物时，结果会被包一层：

```json
{
  "generatedAt": "2026-08-04T13:06:12+08:00",
  "artifactCount": 2,
  "artifacts": [{ "platform": "android", "format": "apk", "...": "..." }],
  "failures": [{ "path": "dist/broken.apk", "error": "Not a readable Android package" }]
}
```

只指定单个产物时，输出仍然只是该产物本身的结果，与此前一致。显式指定的路径必须分析
成功，否则命令失败；扫描过程中发现的产物若分析失败，会记录在 `failures` 里，不会让
整次运行前功尽弃。

## HTML 报告

```bash
fastforge analyze dist --output report.html
```

报告是单个自包含文件——不引用外部样式、脚本或字体——可以直接打开，也可以作为构建
产物附带。

开头是一行统计，以及三张分布图：按运行时、按签名状态、以及体积最大的产物。点击某个
条目即可筛选下方全部内容，多个筛选条件可叠加。表格支持按任意列排序，点击某一行会就地
展开，显示该产物的身份信息、技术栈、体积构成、签名，以及完整的分析 JSON。分析失败的
条目列在最后。

页面依据内嵌的分析数据自行渲染，因此需要 JavaScript；需要程序化读取结果时请用 JSON
输出。

`--output report.html` 会根据扩展名选择 HTML；也可以用 `--format html` 直接输出到
stdout，或在 `.html` 输出路径下用 `--format json` 保持 JSON。

## 技术栈

所有支持深度分析的格式都会输出 `techStack`，说明应用构建在什么技术之上：

| 字段                                                                             | 内容                                                                            |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `runtime`                                                                        | `flutter`、`electron`、`react-native`、`unity`、`cordova`、`dotnet`、`qt`、`java` 或 `native` |
| `<runtime>`                                                                      | 该运行时暴露的细节——引擎 revision、构建模式、AOT、插件、JS 引擎、asar 清单等     |
| `languages`                                                                      | 从二进制链接的运行时或包内容推断出的语言                                        |
| `uiToolkits`                                                                     | Apple 平台的 SwiftUI / AppKit / UIKit，Android 的 Jetpack Compose / AppCompat    |
| `toolchain` / `buildTools`                                                       | 平台、部署目标、SDK，以及构建时记录的编译器、链接器或 Gradle 版本               |
| `libraries` / `dependencies`                                                     | Android 包内嵌的 Maven 坐标与版本                                               |
| `systemFrameworks`、`embeddedFrameworks`、`systemLibraries`、`privateFrameworks` | Apple 二进制链接了哪些 framework 和动态库                                       |
| `nativeLibraries`                                                                | Android 包携带的 `.so` 文件                                                     |
| `thirdPartySdks`                                                                 | 识别出的第三方 SDK 及其用途——更新器、崩溃上报、数据分析等                       |

Apple 平台的链接信息来自主可执行文件的 Mach-O load command，因此描述的是应用自身
链接了什么；只经由内嵌 framework 间接引用的代码，会出现在那个 framework 的条目里。

## macOS：`.app` 与 DMG

除 `techStack` 外，bundle 还会给出架构（读自 Mach-O 头部）、体积构成、内嵌组件与
签名状态。

| 字段                                                          | 内容                                                                 |
| ------------------------------------------------------------- | -------------------------------------------------------------------- |
| `architectures`、`universal`                                  | 可执行文件中的架构切片——`arm64`、`arm64e`、`x86_64` 等               |
| `sizeBytes`、`fileCount`、`sizeBreakdown`、`largestFiles`     | 总体积、`Contents` 下各目录的体积、最大的 10 个文件                  |
| `buildInfo`                                                   | `Info.plist` 中记录的 SDK、平台、Xcode 与编译器信息                  |
| `components`                                                  | 内嵌的 framework、动态库、辅助 App、XPC 服务与插件                   |
| `codeSignature`                                               | 签名类型、团队、证书链、加固运行时、entitlements、公证、Gatekeeper   |
| `provisioningProfile`                                         | 名称、团队、分发类型、过期时间                                       |
| `urlSchemes`、`documentTypes`、`privacyUsageDescriptions`     | 应用注册的能力，以及会申请哪些权限                                   |
| `category`、`minOSVersion`、`localizations`、`sandboxed`      | bundle 声明的分发元数据                                              |

DMG 还会额外给出镜像本身的信息，并把 bundle 的分析结果嵌在 `app` 下：

| 字段            | 内容                                                              |
| --------------- | ----------------------------------------------------------------- |
| `codeSignature` | 镜像自身的签名与公证状态                                          |
| `diskImage`     | 格式（`UDZO`、`ULFO` 等）、压缩情况、校验和、分区                 |
| `volume`        | 卷名、内容、`/Applications` 快捷方式、自定义窗口布局、背景、卷图标 |
| `app` / `apps`  | 主 bundle 的完整分析；镜像内有多个 App 时另外给出各自的摘要       |

```bash
fastforge analyze dist/1.0.0+1/example-1.0.0+1-macos.dmg
```

```json
{
  "platform": "macos",
  "format": "dmg",
  "identifier": "com.example.app",
  "version": "1.0.0",
  "sha256": "83bc18419eab947f614e4d3aeb98daa0db9c77365f2c1de2141d76b98375946c",
  "diskImage": { "format": "UDZO", "compressed": true, "encrypted": false },
  "volume": { "name": "Example", "hasApplicationsSymlink": true },
  "app": {
    "architectures": ["x86_64", "arm64"],
    "techStack": {
      "runtime": "flutter",
      "languages": ["Swift", "Objective-C"],
      "toolchain": { "platform": "macOS", "minOS": "12.0", "sdk": "26.5" }
    },
    "codeSignature": { "signingType": "developer-id", "notarization": { "stapled": true } }
  }
}
```

## iOS：IPA

App bundle 直接从压缩包中读取——无需解包，也不要求 macOS 宿主。

| 字段                                                    | 内容                                                                              |
| ------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `architectures`、`minOSVersion`、`deviceFamilies`       | 主二进制的架构切片、部署目标、支持 iPhone / iPad / Vision                         |
| `buildInfo`                                             | `Info.plist` 中记录的 SDK、Xcode 与编译器信息                                     |
| `components`                                            | 内嵌 framework、App Extension（含扩展点）、Watch App                              |
| `capabilities`                                          | URL scheme、文档类型、后台模式、设备能力要求、ATS、隐私用途描述                   |
| `provisioningProfile`                                   | 名称、团队、过期时间、entitlements，以及分发类型——`development`、`ad-hoc`、`enterprise` 或 `app-store` |
| `contents`                                              | 条目数、各目录体积、最大的 10 个条目                                              |
| `codeSignature`                                         | payload 是否带有已封存的资源签名目录                                              |

```json
{
  "platform": "ios",
  "format": "ipa",
  "identifier": "dev.example.app",
  "deviceFamilies": ["iPhone", "iPad"],
  "architectures": ["arm64"],
  "techStack": {
    "runtime": "flutter",
    "flutter": { "aot": true, "plugins": ["url_launcher_ios"] },
    "uiToolkits": ["SwiftUI", "UIKit"],
    "toolchain": { "platform": "iOS", "minOS": "15.0", "sdk": "17.2", "swift": "5.9" }
  },
  "provisioningProfile": { "distributionType": "ad-hoc", "provisionedDeviceCount": 3 }
}
```

## Android：APK 与 AAB

manifest 由 `aapt2`（或 `bundletool`）提供，其余信息都直接读自包内容。

| 字段        | 内容                                                                                          |
| ----------- | --------------------------------------------------------------------------------------------- |
| `abis`      | 包中带有原生代码的 ABI                                                                        |
| `manifest`  | min / target / compile SDK、权限、特性、启动 Activity、语言、密度、屏幕支持                   |
| `techStack` | 运行时、语言、UI 框架、AGP / Gradle / Kotlin 版本、带版本号的 AndroidX 依赖、原生库           |
| `contents`  | 条目数、dex 数量与体积、各目录体积、最大的 10 个条目                                          |
| `signature` | APK：`apksigner` 给出的验证通过的签名方案与证书；AAB：是否经过 JAR 签名                       |
| `modules`   | 仅 AAB——base 模块与各动态特性模块，含体积与内容                                               |

AAB 还记录了构建时解析出的完整依赖图，比 APK 携带的版本标记更完整：

```json
{
  "platform": "android",
  "format": "aab",
  "abis": ["arm64-v8a"],
  "manifest": { "minSdkVersion": 24, "targetSdkVersion": 35 },
  "techStack": {
    "runtime": "flutter",
    "languages": ["Kotlin", "Dart", "C/C++"],
    "buildTools": { "androidGradlePlugin": "8.7.2", "gradle": "8.9", "kotlin": "2.1.0" },
    "dependencies": [{ "name": "androidx.core:core", "version": "1.17.0" }]
  },
  "modules": [{ "name": "base", "dexCount": 1 }, { "name": "premium", "dexCount": 1 }]
}
```

`contents.sizeBreakdown` 使用压缩后的体积，因为那才是下载成本；`largestEntries`
同时给出压缩前后的体积。

## AAB 的 bundletool 回退

找不到可用 `aapt2` 时，可以通过 `BUNDLETOOL` 指向 bundletool JAR：

```bash
export BUNDLETOOL=/path/to/bundletool.jar
fastforge analyze dist/app-release.aab
```

## CI 用法

```bash
fastforge analyze "$ARTIFACT" --output artifact-metadata.json
```

命令遇到不支持的扩展名、缺少工具或无法解析的产物时会以非零状态退出。

## 注意事项

- 产物不带某项元数据时，对应字段会被省略而不是输出 `null`，因此结构会随输入变化。
- Apple 产物的 `buildNumber` 是字符串，因为 `CFBundleVersion` 并不总是数字；Android
  的 version code 仍是整数。
- macOS 签名分析依赖 `codesign`；Gatekeeper（`spctl`）与公证（`xcrun stapler`）只对
  已签名的产物执行，且可能访问网络。所有外部命令都有 30 秒上限，缺少某个工具时只会
  省略对应字段。
- 加密的磁盘镜像会被拒绝分析，因为挂载它会阻塞在密码输入上。
