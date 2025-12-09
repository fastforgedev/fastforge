# 分发选项

## 示例

```yaml
variables:
  PGYER_API_KEY: 'your api key'
output: dist/
releases:
  - name: dev
    jobs:
      # 构建 apk 包并将其发布到 pgyer
      - name: release-dev-android
        package:
          platform: android
          target: apk
          build_args:
            flavor: dev
            target-platform: android-arm,android-arm64
            dart-define:
              APP_ENV: dev
        publish_to: pgyer
      # 构建 ipa 包并将其发布到 pgyer
      - name: release-dev-ios
        package:
          platform: ios
          target: ipa
          build_args:
            flavor: dev
            export-options-plist: ios/dev_ExportOptions.plist
            dart-define:
              APP_ENV: dev
        publish_to: pgyer
      # 构建 apk 包并指定参数(可选)发布到 pyger
      # 参数说明: https://www.pgyer.com/doc/view/api#fastUploadApp
      - name: release-staging-android
        package:
          platform: android
          target: apk
          build_args:
            flavor: dev
            target-platform: android-arm,android-arm64
            dart-define:
              APP_ENV: dev
        publish:
          target: pgyer
          args:
            pgyer-oversea: 1
            pgyer-install-type: 2
            pgyer-password: 123456
            pgyer-description: 你的应用说明
            pgyer-update-description: 更新说明
            pgyer-install-date: 1
            pgyer-install-start-date: 2025-09-30
            pgyer-install-end-date: 2025-10-30
            pgyer-channel-shortcut: XXXXXX
```

## Specification

| Field Name | Type     | Description            |
| ---------- | -------- | ---------------------- |
| `env`      | `map`    | environment variables. |
| `output`   | `string` | e.g. `dist/`           |
| `releases` | -        | -                      |
