# tar.gz

将 Linux、macOS、Windows 或 Web 的 Flutter 应用打包为 gzip 压缩的 TAR 归档，无需安装外部压缩工具。

```bash
fastforge package --platform linux --targets tar.gz
```

产物写入 `dist/<version>/<artifact>.tar.gz`。Linux、Windows 和 Web 归档包含构建目录的内容；macOS 归档包含 `.app` 包。保留文件权限、符号链接、隐藏文件和空目录。再次运行命令会覆盖已有归档。

无需 `make_config.yaml`。构建环境要求与目标平台的其他打包格式相同。
