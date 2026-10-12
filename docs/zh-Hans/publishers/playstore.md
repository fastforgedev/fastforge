# Google Play

[English](../../en/publishers/playstore.md) | 简体中文

`playstore` target 上传 Android App Bundle 到 Google Play，并可将其分配到某个 track。

## 认证

`PLAYSTORE_CREDENTIALS` 必须是服务账号 JSON 文件的**路径**，该服务账号需要拥有目标应用的 Google Play Developer API 权限：

```bash
export PLAYSTORE_CREDENTIALS="$PWD/service-account.json"
```

## 发布

只接受 `.aab` 文件：

```bash
fastforge publish --path dist/app-release.aab --target playstore \
  --publish-arg package-name=com.example.app \
  --publish-arg track=internal
```

| 参数               | 必填 | 说明                                                   |
| ------------------ | :--: | ------------------------------------------------------ |
| `package-name`     |  是  | 应用包名（`--playstore-package-name`）                 |
| `credentials-file` |  否  | 覆盖 `PLAYSTORE_CREDENTIALS`                           |
| `track`            |  否  | 把上传的 version code 分配到该 track（`--playstore-track`）：`production`、`beta`（开放测试）、`alpha`（封闭测试）、`internal`（内部测试）或自定义 track 名 |

发布器会创建 edit、上传 bundle 并提交 edit。设置了 `track` 时，会先更新该 track，写入包含新 version code、状态为 `completed`（全量发布）的 release。文件名按 `-` 分隔后的第二段为 `<version>+<build>` 时（例如 `my_app-1.0.0+5-android.aab`），release 名称为 `<build> (<version>)`；否则使用应用版本，或 `release`。发布结果为该应用的 Play Console 地址。

## 后续管理

track 查询和商店元数据由 `fastforge googleplay` 处理；分阶段发布和版本说明需要使用 `fastforge googleplay api`，或通过 catalog 推送 track 文件。见 [Google Play](../stores/googleplay.md)。
