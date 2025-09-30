# Distribute Options

## Example

```yaml
variables:
  PGYER_API_KEY: 'your api key'
output: dist/
releases:
  - name: dev
    jobs:
      # Build and publish your apk pkg to pgyer
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
      # Build and publish your ipa pkg to pgyer
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
      # Build apk and specify parameters (optional) to publish to pgyer
      # Params reference: https://www.pgyer.com/doc/view/api#fastUploadApp
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
            pgyer-description: Your app description
            pgyer-update-description: Update description
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
