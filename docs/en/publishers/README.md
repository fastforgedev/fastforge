# Publishing Targets

English | [简体中文](../../zh-Hans/publishers/README.md)

This section groups the currently connected targets by purpose. Object storage services share one page to avoid repetition.

## Support Matrix

| Target             | Purpose                        | Documentation                     |
| ------------------ | ------------------------------ | --------------------------------- |
| `s3`, `minio`      | S3-compatible object storage   | [Object storage](s3.md)           |
| `qiniu`            | Qiniu object storage           | [Object storage](s3.md)           |
| `oss`              | Alibaba Cloud OSS              | [Object storage](s3.md)           |
| `cos`              | Tencent Cloud COS              | [Object storage](s3.md)           |
| `fir`              | fir.im app distribution        | [fir.im](fir.md)                  |
| `pgyer`            | PGYER app distribution         | [PGYER](pgyer.md)                 |
| `firebase`         | Firebase App Distribution      | [Firebase](firebase.md)           |
| `firebase-hosting` | Firebase Hosting               | [Firebase](firebase.md)           |
| `github`           | GitHub Releases                | [GitHub](github.md)               |
| `appstore`         | App Store Connect build upload | [App Store](appstore.md)          |
| `playstore`        | Google Play AAB upload         | [Google Play](playstore.md)       |
| `appgallery`       | Huawei AppGallery              | [AppGallery](appgallery.md)       |
| `vercel`           | Vercel Production              | [Vercel](vercel.md)               |
| `custom`           | Custom shell command           | [Custom](custom.md)               |

## General Usage

```bash
fastforge publish \
  --path <artifact> \
  --target <target> \
  --publish-arg key=value
```

Prefer process environment variables for credentials. Do not commit secrets to the repository.

Uploading a build is not the same as releasing it in a store: review submission, staged rollouts, and store metadata are handled by `fastforge appstore`, `fastforge googleplay`, and `fastforge appgallery`; see [Stores](../stores/README.md).

Use [Local Workflows](../workflows.md) to combine multiple steps.
