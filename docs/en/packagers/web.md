# Web

English | [简体中文](../../zh-Hans/packagers/web.md)

Fastforge packages Flutter web builds as a ZIP archive or a direct copy of the build directory.

Also supports [tar.gz](tar-gz.md) archives without external compression tools.

## Current Status

| Build system    | `package` status                   |
| --------------- | ---------------------------------- |
| Flutter Builder | ZIP and direct through CLI/action |

Web packaging requires a Flutter project (`pubspec.yaml`) and runs on any host with the Flutter SDK. Because both formats are shared with other platforms, pass `--platform web` unless the project layout makes the platform unambiguous.

```bash
fastforge package --platform web --targets zip
```

- `zip` archives the contents of `build/web/` into `dist/<version>/<artifact>.zip`.
- `direct` copies `build/web/` to a directory under `dist/<version>/`.

Neither format needs extra tools. The result can be deployed with publishers such as [Vercel](../publishers/vercel.md) or [S3](../publishers/s3.md).

Return to the [packager overview](README.md).
