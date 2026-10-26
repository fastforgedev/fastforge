# macOS

[English](../../en/packagers/macos.md) | 简体中文

Fastforge 支持把 macOS 应用打包为 [DMG](#dmg)、[PKG](#pkg) 或 [ZIP](#zip)。

## 当前状态

| 构建系统        | `package` 状态                         |
| --------------- | -------------------------------------- |
| Xcode           | DMG、PKG、ZIP；推荐使用 package action |
| Flutter Builder | DMG、PKG、ZIP 已通过 CLI/action 接入   |

Fastforge 根据项目文件选择 Xcode Builder 或 Flutter Builder，随后把生成的 `.app` 交给对应打包器。Flutter 项目可以直接使用 `fastforge package`，并且多个格式只构建一次：

```bash
fastforge package --targets dmg,pkg,zip
```

Xcode 项目的工程参数建议通过工作流 `build-args` 传入，见 [Xcode Builder](../builders/xcode.md#macos)。产物写入 `dist/<version>/`。

## 环境要求

- macOS
- Xcode 命令行工具
- 项目自身需要的 SDK 和构建工具
- ZIP 需要 `PATH` 中有 `7z`

## DMG

DMG 是 macOS 常用的磁盘映像分发格式。当前版本使用内置 DMG maker，不再要求全局安装 `appdmg` Node.js 工具。

```bash
fastforge package --targets dmg
```

`macos/packaging/dmg/` 中的内容（背景图、图标等）会被复制到 `.app` 旁边。如果存在 `macos/packaging/dmg/make_config.yaml`，它会作为 appdmg 风格的规格使用，路径相对于该目录。可用的键为 `title` 和 `contents`（均为必填）、`icon`、`background`、`background-color`、`icon-size`、`window`（`position`、`size`）、`format`（默认 `UDZO`）、`filesystem`（`HFS+` 或 `APFS`），以及 `code-sign`（`signing-identity`，可选 `identifier` 和 `timestamp`），后者会用 `codesign` 为 DMG 签名。Fastforge 还会读取 `notarize`（见[公证](#公证)）。未知的键会导致打包失败；也接受带 `app` 和 `icons` 的旧版 appdmg 格式。否则使用默认布局：应用和 `/Applications` 链接，存在 `background.png` 时一并使用。

```yaml
title: Hello World            # 挂载后显示的卷名
icon: volume.icns             # 卷图标
background: background.png    # 或 background-color: "#f5f5f7"
icon-size: 96
window:
  position: { x: 200, y: 120 }
  size: { width: 640, height: 480 }
format: ULFO                  # UDZO（默认）、UDRO、UDCO、UDBZ、ULFO、ULMO、UDRW
filesystem: APFS              # HFS+（默认）或 APFS
contents:
  - { x: 448, y: 344, type: link, path: /Applications }
  - { x: 192, y: 344, type: file, path: Hello World.app }
  - { x: 100, y: 100, type: file, path: README.txt, name: Read Me.txt }
code-sign:
  signing-identity: "Developer ID Application: Example (TEAMID)"
```

`contents` 中，`file` 把文件或 bundle 复制进镜像（可用 `name` 重命名），`link` 创建符号链接，`position` 只调整已有项目的位置。坐标是图标中心相对窗口左上角的点数。`.app` 条目必须使用 bundle 的实际名称，它由 Xcode 的 `PRODUCT_NAME` 决定，可能与 `pubspec.yaml` 的名称不同；名称不符时打包会失败，并给出实际构建出的 bundle 名。

公证 DMG 见[公证](#公证)。其他流程可以通过 post-package 钩子执行（`--hook-post`，或 release job 中的 `package.hooks.post`）。

## PKG

PKG 是 macOS 安装器包格式。Fastforge 使用 `xcrun productbuild` 生成 PKG，可以是直接下载分发的安装包，也可以是 Mac App Store 安装包。PKG 打包器会读取以下可选配置文件：

```text
macos/packaging/pkg/make_config.yaml
```

| 键              | 说明                                                |
| --------------- | --------------------------------------------------- |
| `install-path`  | 安装位置，默认 `/Applications/`                     |
| `sign-identity` | 安装包签名身份：直接下载用 "Developer ID Installer"，Mac App Store 用 "3rd Party Mac Developer Installer" 或 "Mac Installer Distribution" |
| `scripts`       | 包含安装前/后脚本的目录（App Store 安装包不可用）   |
| `notarize`      | 为 `true` 时公证并装订安装包，见[公证](#公证)（App Store 安装包不可用） |
| `app-store`     | 为 `true` 时生成 Mac App Store 安装包，见 [Mac App Store 安装包](#mac-app-store-安装包)；默认值取决于 `sign-identity` 是否为 App Store 安装包签名身份 |

文件不存在时使用默认值；文件无法解析时打包失败。

```bash
fastforge package --targets pkg
```

### 直接下载的安装包

未开启 `app-store` 时，Fastforge 生成在商店之外分发的安装包：用 `pkgutil` 展开 productbuild 的输出，把 Distribution 限制为只能安装到启动磁盘（`enable_localSystem`），从组件 PackageInfo 中删除重定位（relocation）条目以保证应用总是安装到 `install-path` 而不是已有副本的位置，再重新打平。`sign-identity` 通过 `xcrun productsign` 签名，`scripts` 加入安装脚本，`notarize` 公证并装订。

### Mac App Store 安装包

App Store Connect 会按 productbuild 写出的内容校验安装包，事后被修改过的会被拒绝（ITMS-90229 "Bundle ... in Payload not declared in PackageInfo"、ITMS-90270 "Unsupported toolchain"）。因此 `app-store: true` 时，Fastforge 只运行一次 `xcrun productbuild --component <App>.app <install-path> --sign <sign-identity> <artifact>.pkg`，并原样交付其输出：不经过 `pkgutil` 展开/打平，不修改 Distribution 或 PackageInfo，也不再单独 `productsign`。这与 `xcodebuild -exportArchive` 以 `app-store-connect` 方式导出时运行的命令相同，结果也与 Xcode 的导出一致：安装路径按 Xcode 的做法去掉末尾斜杠传入，组件 PackageInfo 只列出应用 bundle（productbuild 不会声明嵌套的应用扩展或资源 bundle，Apple 接受的 Xcode 安装包同样如此）。ITMS-90229 和 ITMS-90270 的拒绝源于事后改写，而不是安装包内容。

当 `sign-identity` 以 `3rd Party Mac Developer Installer` 或 `Mac Installer Distribution` 开头时自动进入 App Store 模式；设置 `app-store: false` 可在使用这类身份时保留直接下载的行为，`app-store: true` 则对任意身份强制开启。该模式下 `sign-identity` 为必填，`scripts` 和 `notarize`（包括 `--notarize`）会使打包失败：商店安装包不能携带安装脚本，也不需要公证。`.app` 必须已由构建按商店要求签名（"Apple Distribution" 或 "3rd Party Mac Developer Application" 身份，并带 Mac App Store 描述文件）。

```yaml
# macos/packaging/pkg/make_config.yaml
install-path: /Applications/
sign-identity: "3rd Party Mac Developer Installer: Example (TEAMID)"
app-store: true   # 使用该身份时可省略
```

打包并上传：

```bash
fastforge package --platform macos --targets pkg
fastforge publish --path dist/<version>/<artifact>.pkg --targets appstore
```

## ZIP

ZIP 使用 `7z` 将 macOS `.app` bundle 压缩为便于下载和发布的归档文件。可选的 `macos/packaging/zip/make_config.yaml` 支持 `notarize: true`（见[公证](#公证)）。

```bash
fastforge package --platform macos --targets zip
```

打包后可以直接发布，适合 GitHub Releases、S3 兼容存储或自定义下载服务：

```bash
fastforge publish --path dist/<version>/<artifact>.zip --targets github \
  --publish-arg repo=owner/repository \
  --publish-arg release-tag=v1.0.0
```

## 公证

在 Mac App Store 之外分发的应用必须用 Developer ID 签名并经过公证，否则 Gatekeeper 会在首次启动时阻止或警告。Fastforge 可以用 `xcrun notarytool` 公证 DMG、PKG 和 ZIP 产物，并用 `xcrun stapler` 装订公证票据。该功能默认关闭；用 `--notarize` 为所有 macOS 目标开启：

```bash
fastforge package --platform macos --targets dmg,zip --notarize
```

也可以在对应格式的 `macos/packaging/<dmg|pkg|zip>/make_config.yaml` 中设置 `notarize: true`，只为该格式开启（ZIP 打包器的这个文件只读取这一个键）。在工作流中，为 `fastforge/package` 设置输入 `notarize: "true"`。

Fastforge 不会为应用签名：构建产出的 `.app` 必须已经用 "Developer ID Application" 身份签名，并启用 hardened runtime、带有安全时间戳（在 Xcode 中配置 macOS Release 签名）。提交前会先检查这些条件，缺少某项时几秒内就会失败并给出说明，而不必等一次公证往返。

凭据从环境变量读取，CI 无需修改配置。按顺序使用第一组完整的凭据：

| 凭据                          | 环境变量                                                                                       |
| ----------------------------- | ---------------------------------------------------------------------------------------------- |
| 已保存的 `notarytool` 配置    | `NOTARYTOOL_KEYCHAIN_PROFILE`（用 `xcrun notarytool store-credentials` 保存）                   |
| App Store Connect API 密钥    | `APP_STORE_CONNECT_KEY_ID`、`APP_STORE_CONNECT_KEY_PATH`（`.p8`）、`APP_STORE_CONNECT_ISSUER_ID`（个人密钥可省略） |
| Apple ID                      | `APPLE_ID`、`APPLE_APP_SPECIFIC_PASSWORD`、`APPLE_TEAM_ID`                                      |

API 密钥变量与 `fastforge publish --targets appstore` 使用的相同。使用 `--host` 时，这些变量需要设置在远程主机上。

各格式的处理：

- **ZIP**：公证 `.app`（以 `ditto` 生成的 zip 提交）并装订，再压缩已装订的应用。ZIP 本身无法装订。
- **DMG**：公证并装订 `.app`，用已装订的应用制作镜像，以 `--timestamp` 为镜像签名，再公证并装订 DMG。`make_config.yaml` 中没有 `code-sign` 时，使用应用的 Developer ID Application 身份为镜像签名。
- **PKG**：需要 `sign-identity`（"Developer ID Installer" 身份）；用 `productsign --timestamp` 签名后公证并装订安装包。Mac App Store 安装包（`app-store: true`）不公证，为其设置 `notarize` 会使打包失败。

`.app` 会在构建输出中原地装订，因此 `--targets dmg,zip` 只公证一次应用，两个归档中都是已装订的应用。`notarytool submit --wait` 通常几分钟内返回，但每次提交（应用、DMG、PKG）都会增加打包时间。提交结果为 `Invalid` 或 `Rejected` 时，Fastforge 会获取公证日志、打印其中的问题并使打包失败。可以用 [`fastforge analyze`](../tools/analyze.md) 检查结果，它会报告签名、装订和 Gatekeeper 状态。钩子仍会照常运行，可用于其他自定义步骤。

返回[打包器总览](README.md)。
