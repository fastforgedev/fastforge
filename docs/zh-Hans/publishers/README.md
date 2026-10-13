# 发布目标

[English](../../en/publishers/README.md) | 简体中文

本节按用途拆分当前实际接通的 target。对象存储服务合并为一页，减少重复。

## 支持矩阵

| Target             | 用途                       | 文档                         |
| ------------------ | -------------------------- | ---------------------------- |
| `s3`、`minio`      | S3 兼容对象存储            | [对象存储](s3.md)            |
| `qiniu`            | 七牛云对象存储             | [对象存储](s3.md)            |
| `oss`              | 阿里云 OSS                 | [对象存储](s3.md)            |
| `cos`              | 腾讯云 COS                 | [对象存储](s3.md)            |
| `fir`              | fir.im 应用分发            | [fir.im](fir.md)             |
| `pgyer`            | 蒲公英应用分发             | [蒲公英](pgyer.md)           |
| `firebase`         | Firebase App Distribution  | [Firebase](firebase.md)      |
| `firebase-hosting` | Firebase Hosting           | [Firebase](firebase.md)      |
| `github`           | GitHub Releases            | [GitHub](github.md)          |
| `gitea`            | Gitea/Forgejo/Codeberg Releases | [Gitea](gitea.md)           |
| `appstore`         | App Store Connect 构建上传 | [App Store](appstore.md)     |
| `playstore`        | Google Play AAB 上传       | [Google Play](playstore.md)  |
| `appgallery`       | Huawei AppGallery          | [AppGallery](appgallery.md)  |
| `vercel`           | Vercel Production          | [Vercel](vercel.md)          |
| `apt-repo`         | 目录中的 APT 软件源        | [软件源](repositories.md)    |
| `pacman-repo`      | 目录中的 Pacman 软件源     | [软件源](repositories.md)    |
| `custom`           | 自定义 shell 命令          | [Custom](custom.md)          |

## 通用用法

```bash
fastforge publish \
  --path <artifact> \
  --target <target> \
  --publish-arg key=value
```

凭证优先使用进程环境变量，不要把密钥写入版本库。

上传构建不等于在商店中发布：提交审核、分阶段发布和商店元数据由 `fastforge appstore`、`fastforge googleplay` 与 `fastforge appgallery` 处理，见[商店](../stores/README.md)。

需要组合多个步骤时，使用[本地工作流](../workflows.md)。
