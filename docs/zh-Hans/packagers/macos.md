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

`macos/packaging/dmg/` 中的内容（背景图、图标等）会被复制到 `.app` 旁边。如果存在 `macos/packaging/dmg/make_config.yaml`，它会作为 appdmg 风格的规格使用，路径相对于该目录。可用的键为 `title` 和 `contents`（均为必填）、`icon`、`background`、`background-color`、`icon-size`、`window`（`position`、`size`）、`format`（默认 `UDZO`）、`filesystem`（`HFS+` 或 `APFS`），以及 `code-sign`（`signing-identity`，可选 `identifier`），后者会用 `codesign` 为 DMG 签名。未知的键会导致打包失败；也接受带 `app` 和 `icons` 的旧版 appdmg 格式。否则使用默认布局：应用和 `/Applications` 链接，存在 `background.png` 时一并使用。

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

公证等额外流程可以通过 post-package 钩子执行（`--hook-post`，或 release job 中的 `package.hooks.post`）。

## PKG

PKG 是 macOS 安装器包格式。安装包只能安装到启动磁盘（`enable_localSystem`）。Fastforge 使用 `xcrun productbuild` 和 `pkgutil` 生成 PKG。PKG 打包器会读取以下可选配置文件：

```text
macos/packaging/pkg/make_config.yaml
```

| 键              | 说明                                                |
| --------------- | --------------------------------------------------- |
| `install-path`  | 安装位置，默认 `/Applications/`                     |
| `sign-identity` | 安装包签名身份，通过 `xcrun productsign` 签名       |
| `scripts`       | 包含安装前/后脚本的目录                             |

文件不存在时使用默认值；文件无法解析时打包失败。

```bash
fastforge package --targets pkg
```

需要上传到 App Store 时：

```bash
fastforge publish --path dist/<version>/<artifact>.pkg --targets appstore
```

## ZIP

ZIP 使用 `7z` 将 macOS `.app` bundle 压缩为便于下载和发布的归档文件。

```bash
fastforge package --platform macos --targets zip
```

打包后可以直接发布，适合 GitHub Releases、S3 兼容存储或自定义下载服务：

```bash
fastforge publish --path dist/<version>/<artifact>.zip --targets github \
  --publish-arg repo=owner/repository \
  --publish-arg release-tag=v1.0.0
```

返回[打包器总览](README.md)。
