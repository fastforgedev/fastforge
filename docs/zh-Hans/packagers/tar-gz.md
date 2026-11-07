# tar.gz

[English](../../en/packagers/tar-gz.md) | 简体中文

将 Linux、macOS、Windows 或 Web 应用打包为 gzip 压缩的 TAR 归档。压缩在 Fastforge 内部完成，无需安装 `zip`、`7z`、`tar` 或 `gzip` 命令。

```bash
fastforge package --platform linux --targets tar.gz
```

产物沿用默认命名模板，扩展名为 `.tar.gz`，例如 `dist/1.0.0/demo-1.0.0-linux.tar.gz`。已有输出会被覆盖。

- Linux、Windows 和 Web 归档直接包含构建目录的内容，不额外包一层目录。
- macOS 归档包含 `.app` 包，并保留目录结构和符号链接。Rust CLI 支持用 `--notarize` 在归档前公证并装订应用。
- 保留文件权限（包括可执行位）、符号链接、隐藏文件和空目录。Rust CLI 与 ZIP 一样，排除 Windows 构建输出中的 PDB 调试符号。

构建仍需要目标平台对应的 SDK 和宿主系统。Flutter 项目支持这四个平台，原生 Xcode 项目支持 macOS，原生 .NET 项目支持 Windows。

返回[打包器总览](README.md)。
