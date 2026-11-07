# Web

[English](../../en/packagers/web.md) | 简体中文

Fastforge 支持把 Flutter Web 构建结果打包为 ZIP 归档，或直接复制构建目录。

也支持 [tar.gz](tar-gz.md) 归档，无需安装外部压缩工具。

## 当前状态

| 构建系统        | `package` 状态                    |
| --------------- | --------------------------------- |
| Flutter Builder | ZIP、direct 已通过 CLI/action 接入 |

Web 打包需要 Flutter 项目（含 `pubspec.yaml`），可以在任何安装了 Flutter SDK 的宿主上执行。由于这两种格式也被其他平台使用，除非项目结构能明确推断平台，否则请传入 `--platform web`。

```bash
fastforge package --platform web --targets zip
```

- `zip` 把 `build/web/` 中的内容压缩为 `dist/<version>/<artifact>.zip`。
- `direct` 把 `build/web/` 复制到 `dist/<version>/` 下的目录。

两种格式都不需要额外工具。产物可以使用 [Vercel](../publishers/vercel.md) 或 [S3](../publishers/s3.md) 等发布器部署。

返回[打包器总览](README.md)。
