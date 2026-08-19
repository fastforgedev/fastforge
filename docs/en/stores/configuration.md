# Store Configuration

English | [简体中文](../../zh-Hans/stores/configuration.md)

`.fastforge/config.yaml` registers App Store, AppGallery, and Google Play apps for the aggregated `fastforge store` commands.

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

## App Store Fields

| Field                             | Description                                                                |
| --------------------------------- | -------------------------------------------------------------------------- |
| `auth.key_id`                     | App Store Connect API Key ID                                               |
| `auth.issuer_id`                  | Issuer ID                                                                  |
| `auth.key_path`                   | Path to the `.p8` private key                                              |
| `auth.username` / `auth.password` | Compatible username and app-specific-password authentication               |
| `apps[].bundle_id`                | Preferred application identifier                                           |
| `apps[].app_id`                   | Fallback identifier used by catalog commands when the bundle ID is missing |
| `apps[].sku` / `apps[].name`      | Optional metadata                                                          |

## Google Play Fields

| Field                       | Description                           |
| --------------------------- | ------------------------------------- |
| `auth.service_account_key`  | Path to the service-account JSON file |
| `auth.service_account_json` | Service-account JSON content          |
| `apps[].package_name`       | Google Play package name              |
| `apps[].track`              | Optional default track                |

## AppGallery Fields

| Field                       | Description                                      |
| --------------------------- | ------------------------------------------------ |
| `auth.service_account_key`  | Path to the AppGallery service-account JSON file |
| `auth.service_account_json` | Service-account JSON content                     |
| `auth.client_id`            | Legacy API client ID                             |
| `auth.client_secret`        | Legacy API client secret                         |
| `apps[].app_id`             | AppGallery application ID                        |
| `apps[].package_name`       | Android package name                             |
| `apps[].name`               | Optional display name                            |

`auth` fields support complete `${ENV_NAME}` references and also read default environment variables. The current store API and catalog executors still establish authentication from process environment variables, so export credentials before running commands.

## Security Recommendations

- Store real credentials in CI secrets or local environment variables.
- Do not commit `.p8` files, service-account JSON, or passwords to Git.
- Application identifier configuration without secrets can be committed.
- Add the configuration file to `.gitignore` if it contains real secrets.
