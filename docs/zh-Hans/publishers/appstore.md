# App Store

[English](../../en/publishers/appstore.md) | 简体中文

`appstore` target 使用 macOS `xcrun altool` 上传 IPA 或 PKG 到 App Store Connect。

## 环境要求

- macOS 与 Xcode 命令行工具
- 已正确签名的 `.ipa` 或 `.pkg`（`.ipa` 以 `ios` 类型上传，其他文件以 `osx` 类型上传）
- 以下两种认证方式之一

## API Key 认证

```bash
export APP_STORE_CONNECT_KEY_ID=ABC123DEFG
export APP_STORE_CONNECT_ISSUER_ID=00000000-0000-0000-0000-000000000000
export APP_STORE_CONNECT_KEY_PATH="$PWD/AuthKey_ABC123DEFG.p8"
```

key ID 与 issuer ID 必须同时提供。兼容变量 `APPSTORE_APIKEY`、`APPSTORE_APIISSUER` 会被优先读取。

`APP_STORE_CONNECT_KEY_PATH` 为可选。未设置时，`altool` 会在默认的 `private_keys` 目录中查找 `AuthKey_<key id>.p8`；设置后，Fastforge 会把密钥复制到临时的 `private_keys` 目录中完成上传。

## 用户名认证

```bash
export APPSTORE_USERNAME=user@example.com
export APPSTORE_PASSWORD=app-specific-password
```

用户名与 App 专用密码必须同时提供。

## 上传

```bash
fastforge publish --path dist/MyApp.ipa --target appstore
```

发布器也接受 `key-id`（或 `api-key`）、`issuer-id`（或 `api-issuer`）、`key-path`、`username`、`password` 参数。敏感凭证不建议这样传递，以免写入命令历史。发布结果为 `https://appstoreconnect.apple.com/apps`。

## 后续管理

上传完成不等于提交审核。查询构建、关联版本和创建审核 submission 请使用 `fastforge appstore`，见 [App Store Connect](../stores/appstore.md)。
