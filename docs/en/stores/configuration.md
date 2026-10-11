# Store Configuration

English | [简体中文](../../zh-Hans/stores/configuration.md)

`.fastforge/config.yaml` registers App Store, AppGallery, and Google Play apps for the aggregated `fastforge store` commands. `fastforge store list` shows all three stores; `fastforge store catalog pull|push` processes only App Store and Google Play apps.

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

## App Store Fields

| Field                             | Description                                                                |
| --------------------------------- | -------------------------------------------------------------------------- |
| `auth.key_id`                     | App Store Connect API Key ID                                               |
| `auth.issuer_id`                  | Issuer ID                                                                  |
| `auth.key_path`                   | Path to the `.p8` private key                                              |
| `auth.username` / `auth.password` | Compatible username and app-specific-password authentication               |
| `apps[].bundle_id`                | Preferred application identifier                                           |
| `apps[].app_id`                   | Fallback identifier used by catalog commands when the bundle ID is missing; `store list` shows only `bundle_id` |
| `apps[].sku` / `apps[].name`      | Optional metadata                                                          |
| `apps[].platform`                 | App Store platform (`IOS`, `MAC_OS`, `TV_OS`, or `VISION_OS`); defaults to `IOS` |

## Google Play Fields

| Field                       | Description                           |
| --------------------------- | ------------------------------------- |
| `auth.service_account_key`  | Path to the service-account JSON file |
| `auth.service_account_json` | Service-account JSON content          |
| `apps[].package_name`       | Google Play package name              |
| `apps[].track`              | Optional track note; commands currently take `--track` explicitly |

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

`auth` fields support complete `${ENV_NAME}` references; a reference to an unset variable is kept as literal text. Missing or empty fields fall back to these environment variables:

| Store       | Field                  | Fallback environment variables                                     |
| ----------- | ---------------------- | ------------------------------------------------------------------ |
| App Store   | `key_id`               | `APP_STORE_CONNECT_KEY_ID`, `APPSTORE_APIKEY`                      |
| App Store   | `issuer_id`            | `APP_STORE_CONNECT_ISSUER_ID`, `APPSTORE_APIISSUER`                |
| App Store   | `key_path`             | `APP_STORE_CONNECT_KEY_PATH`                                       |
| App Store   | `username`/`password`  | `APPSTORE_USERNAME`, `APPSTORE_PASSWORD`                           |
| AppGallery  | `service_account_key`  | `APP_GALLERY_SERVICE_ACCOUNT_KEY`                                  |
| AppGallery  | `service_account_json` | `APP_GALLERY_SERVICE_ACCOUNT_JSON`                                 |
| AppGallery  | `client_id`/`client_secret` | `APP_GALLERY_CLIENT_ID`, `APP_GALLERY_CLIENT_SECRET`          |
| Google Play | `service_account_key`  | `GOOGLE_PLAY_SERVICE_ACCOUNT_KEY`, `GOOGLE_APPLICATION_CREDENTIALS` |
| Google Play | `service_account_json` | `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON`                                 |

The config `auth` block is only used to report the authentication type and status (for example, in `fastforge store list`). The store API and catalog commands establish authentication from process environment variables only: `APP_STORE_CONNECT_KEY_ID`, `APP_STORE_CONNECT_ISSUER_ID`, and `APP_STORE_CONNECT_KEY_PATH` for App Store; `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON` for Google Play; and `APP_GALLERY_SERVICE_ACCOUNT_JSON`, `APP_GALLERY_SERVICE_ACCOUNT_KEY`, or `APP_GALLERY_CLIENT_ID` plus `APP_GALLERY_CLIENT_SECRET` for AppGallery. Aliases such as `APPSTORE_APIKEY` or `GOOGLE_APPLICATION_CREDENTIALS` are not read by those commands, so export the canonical variables before running them.

Unknown fields in App Store app entries are rejected so misspelled settings do not silently fall back to iOS.

## Security Recommendations

- Store real credentials in CI secrets or local environment variables.
- Do not commit `.p8` files, service-account JSON, or passwords to Git.
- Application identifier configuration without secrets can be committed.
- Add the configuration file to `.gitignore` if it contains real secrets.
