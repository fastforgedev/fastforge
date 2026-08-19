# 商店配置

[English](../../en/stores/configuration.md) | 简体中文

`.fastforge/config.yaml` 用于登记 App Store、AppGallery 与 Google Play 应用，供 `fastforge store` 聚合命令读取。

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
| `apps[].app_id`                   | bundle id 缺失时 catalog 命令使用的回退标识 |
| `apps[].sku` / `apps[].name`      | 可选元数据                                  |

## Google Play 字段

| 字段                        | 说明                     |
| --------------------------- | ------------------------ |
| `auth.service_account_key`  | 服务账号 JSON 文件路径   |
| `auth.service_account_json` | 服务账号 JSON 内容       |
| `apps[].package_name`       | Google Play package name |
| `apps[].track`              | 可选默认轨道信息         |

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

`auth` 字段支持完整的 `${ENV_NAME}` 引用，也会读取默认环境变量。当前商店 API 和 catalog 执行器仍以进程环境变量建立认证上下文，因此运行命令前应导出凭证。

## 安全建议

- 将真实凭证放入 CI secret 或本地环境变量。
- 不要把 `.p8`、服务账号 JSON 或密码提交到 Git。
- 可以提交不含秘密的应用标识配置。
- 若配置文件包含真实秘密，应加入 `.gitignore`。
