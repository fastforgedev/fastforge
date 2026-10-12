# fir.im

[English](../../en/publishers/fir.md) | 简体中文

`fir` target 上传 Android APK 或 iOS IPA 到 fir.im。

## 配置

```bash
export FIR_API_TOKEN=fir-api-token
```

## 发布

```bash
fastforge publish --path dist/app.apk --target fir
```

Fastforge 会从 APK 或 IPA 中读取 bundle ID、应用名称、版本名和构建号。发布结果为新版本在 fir.im 的下载地址。

## 可选参数

显式传入的参数会覆盖从安装包读取的值：

| 参数           | 说明                       |
| -------------- | -------------------------- |
| `bundle_id`    | Bundle ID / Application ID |
| `app_name`     | 应用显示名称               |
| `version`      | 版本名                     |
| `build_number` | 构建号                     |

`changelog` 设置该版本的更新说明，例如 `--publish-arg changelog="修复启动崩溃"`。

也接受连字符形式（`bundle-id`、`app-name`、`build-number`）。如果无法解析安装包，只有提供了 `bundle_id` 时才会继续上传。

平台只会从 `.apk` 和 `.ipa` 扩展名推断，其他扩展名会失败。
