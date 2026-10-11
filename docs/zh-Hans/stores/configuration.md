# 商店配置

[English](../../en/stores/configuration.md) | 简体中文

`.fastforge/config.yaml` 用于登记 App Store、AppGallery 与 Google Play 应用，供 `fastforge store` 聚合命令读取。`fastforge store list` 会显示三个商店；`fastforge store catalog pull|push` 只处理 App Store 与 Google Play 应用。

```yaml
stores:
  appstore:
    auth:
      key_id: "${APP_STORE_CONNECT_KEY_ID}"
      issuer_id: "${APP_STORE_CONNECT_ISSUER_ID}"
      key_path: "${APP_STORE_CONNECT_KEY_PATH}"
    apps:
      - bundle_id: com.example.myapp
        app_id: "1234567890"
        sku: MYAPP
        name: My App
        platform: MAC_OS

  appgallery:
    auth:
      service_account_key: /secure/appgallery-private.json
    apps:
      - app_id: "987654321"
        package_name: com.example.myapp

  googleplay:
    auth:
      service_account_key: "${GOOGLE_PLAY_SERVICE_ACCOUNT_KEY}"
    apps:
      - package_name: com.example.myapp
        track: production
```

## App Store 字段

| 字段                              | 说明                                        |
| --------------------------------- | ------------------------------------------- |
| `auth.key_id`                     | App Store Connect API Key ID                |
| `auth.issuer_id`                  | Issuer ID                                   |
| `auth.key_path`                   | `.p8` 私钥路径                              |
| `auth.username` / `auth.password` | 用户名与 app-specific password 兼容认证     |
| `apps[].bundle_id`                | 首选应用标识                                |
| `apps[].app_id`                   | bundle id 缺失时 catalog 命令使用的回退标识；`store list` 只显示 `bundle_id` |
| `apps[].sku` / `apps[].name`      | 可选元数据                                  |
| `apps[].platform`                 | App Store 平台：`IOS`、`MAC_OS`、`TV_OS` 或 `VISION_OS`；默认 `IOS` |

## Google Play 字段

| 字段                        | 说明                     |
| --------------------------- | ------------------------ |
| `auth.service_account_key`  | 服务账号 JSON 文件路径   |
| `auth.service_account_json` | 服务账号 JSON 内容       |
| `apps[].package_name`       | Google Play package name |
| `apps[].track`              | 可选 track 备注；命令目前需显式传入 `--track` |

## AppGallery 字段

| 字段                        | 说明                                      |
| --------------------------- | ----------------------------------------- |
| `auth.service_account_key`  | AppGallery Service Account JSON 文件路径  |
| `auth.service_account_json` | Service Account JSON 内容                 |
| `auth.client_id`            | 旧版 API Client ID                        |
| `auth.client_secret`        | 旧版 API Client 密钥                      |
| `apps[].app_id`             | AppGallery 应用 ID                        |
| `apps[].package_name`       | Android 包名                              |
| `apps[].name`               | 可选显示名称                              |

`auth` 字段支持完整的 `${ENV_NAME}` 引用；引用的变量未设置时会保留原文。字段缺失或为空时会回退读取以下环境变量：

| 商店        | 字段                   | 回退环境变量                                                       |
| ----------- | ---------------------- | ------------------------------------------------------------------ |
| App Store   | `key_id`               | `APP_STORE_CONNECT_KEY_ID`、`APPSTORE_APIKEY`                      |
| App Store   | `issuer_id`            | `APP_STORE_CONNECT_ISSUER_ID`、`APPSTORE_APIISSUER`                |
| App Store   | `key_path`             | `APP_STORE_CONNECT_KEY_PATH`                                       |
| App Store   | `username`/`password`  | `APPSTORE_USERNAME`、`APPSTORE_PASSWORD`                           |
| AppGallery  | `service_account_key`  | `APP_GALLERY_SERVICE_ACCOUNT_KEY`                                  |
| AppGallery  | `service_account_json` | `APP_GALLERY_SERVICE_ACCOUNT_JSON`                                 |
| AppGallery  | `client_id`/`client_secret` | `APP_GALLERY_CLIENT_ID`、`APP_GALLERY_CLIENT_SECRET`          |
| Google Play | `service_account_key`  | `GOOGLE_PLAY_SERVICE_ACCOUNT_KEY`、`GOOGLE_APPLICATION_CREDENTIALS` |
| Google Play | `service_account_json` | `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON`                                 |

配置中的 `auth` 仅用于报告认证类型和状态（例如 `fastforge store list`）。商店 API 与 catalog 命令只从进程环境变量建立认证：App Store 使用 `APP_STORE_CONNECT_KEY_ID`、`APP_STORE_CONNECT_ISSUER_ID` 和 `APP_STORE_CONNECT_KEY_PATH`；Google Play 使用 `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON`；AppGallery 使用 `APP_GALLERY_SERVICE_ACCOUNT_JSON`、`APP_GALLERY_SERVICE_ACCOUNT_KEY`，或 `APP_GALLERY_CLIENT_ID` 加 `APP_GALLERY_CLIENT_SECRET`。这些命令不会读取 `APPSTORE_APIKEY`、`GOOGLE_APPLICATION_CREDENTIALS` 等别名，运行前请导出上述标准变量。

App Store 应用配置中的未知字段会直接报错，避免拼写错误被静默忽略并回退到 iOS。

## 安全建议

- 将真实凭证放入 CI secret 或本地环境变量。
- 不要把 `.p8`、服务账号 JSON 或密码提交到 Git。
- 可以提交不含秘密的应用标识配置。
- 若配置文件包含真实秘密，应加入 `.gitignore`。
