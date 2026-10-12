# 安装

[English](../en/installation.md) | 简体中文

Fastforge CLI 以独立二进制运行。CLI 本身不附带各平台的 SDK、构建工具、签名工具或第三方发布客户端。

## 使用安装脚本

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.sh | sh
```

默认安装到 `/usr/local/bin/fastforge`。可以指定安装目录或版本（不带 `v` 前缀），环境变量需要写在管道后的 `sh` 一侧：

```bash
curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.sh \
  | FASTFORGE_INSTALL_DIR="$HOME/.local/bin" sh

curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.sh \
  | FASTFORGE_VERSION="0.7.0" sh
```

安装脚本支持：

- macOS：Apple Silicon、Intel
- Linux：AArch64、x86_64（GNU libc）

如果目标目录不可写，脚本会请求 `sudo` 权限。脚本查询最新版本时，可设置 `GITHUB_TOKEN` 避免 GitHub API 限流。

### Windows

在 PowerShell 中执行：

```powershell
iwr https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.ps1 | iex
```

脚本支持 x86_64 和 ARM64。默认安装到 `%LOCALAPPDATA%\fastforge\bin`，并写入当前用户的 `PATH`。可以通过环境变量指定版本或目录：

```powershell
$env:FASTFORGE_VERSION = "0.7.0"
$env:FASTFORGE_INSTALL_DIR = "$HOME\bin"
iwr https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.ps1 | iex
```

## 从源码安装

从源码安装需要最新稳定版 Cargo 工具链：

```bash
git clone https://github.com/fastforgedev/fastforge.git
cd fastforge
cargo install --path apps/cli
```

开发时可以不安装，直接在仓库根目录运行：

```bash
cargo run -p fastforge_cli -- --help
```

## 验证安装

```bash
fastforge --version
fastforge --help
```

如果终端找不到命令，请确认安装目录已加入 `PATH`。

从源码构建的二进制启动时会在 stderr 打印 `UNOFFICIAL BUILD` 提示，官方发布的二进制不会。

## 升级

```bash
fastforge upgrade
fastforge upgrade --force   # 即使已是最新版本也重新安装
```

`upgrade` 会下载当前平台的最新发布包，并原地替换正在运行的二进制。如果二进制所在目录不可写（例如 `/usr/local/bin`），请使用 `sudo`（Windows 上使用管理员终端）重新运行，或重新执行安装脚本。如需安装指定版本，请带上 `FASTFORGE_VERSION` 重新执行安装脚本。

默认情况下，每个命令运行前都会查询 GitHub Releases 是否有新版本，并在 stderr 打印提示（5 秒超时，不会导致命令失败）。在 CI 等场景可传入 `--no-version-check` 跳过。`fastforge version-check` 可按需执行检查；`fastforge version-check --current-only` 只打印本地版本，不访问网络。

## 项目工具链

根据实际操作准备依赖：

| 场景               | 额外依赖                                                    |
| ------------------ | ----------------------------------------------------------- |
| Android 构建或分析 | Android SDK；APK 分析需要 `ANDROID_HOME`（或 `ANDROID_SDK_ROOT`）下可用的 `aapt2` |
| AAB 分析           | Android SDK 的 `aapt2`，或通过 `BUNDLETOOL` 指定 bundletool |
| iOS / macOS 构建   | macOS、Xcode 和命令行工具                                   |
| 原生 Windows 构建  | Windows 和 .NET SDK（`PATH` 中有 `dotnet`）；WinUI 3 还需要 Windows App SDK |
| Flutter 构建       | Flutter SDK 和目标平台工具链，且 `flutter` 位于 `PATH`      |
| App Store 上传     | macOS、`xcrun` 和有效的 App Store Connect 凭证              |
| Firebase 发布      | Firebase CLI                                                |
| Vercel 发布        | Vercel CLI                                                  |

## 卸载

macOS / Linux：

```bash
curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/uninstall.sh \
  | sh
```

Windows：

```powershell
iwr https://raw.githubusercontent.com/fastforgedev/fastforge/main/uninstall.ps1 | iex
```

若安装时使用了 `FASTFORGE_INSTALL_DIR`，卸载时应传入相同的值。
