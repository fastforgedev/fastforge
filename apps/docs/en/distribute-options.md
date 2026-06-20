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

## Hooks

Package hooks allow you to run shell commands before and after the packaging step of a job.

### Example

```yaml
releases:
  - name: dev
    jobs:
      - name: android-apk
        package:
          platform: android
          target: apk
          hooks:
            pre: echo "Starting packaging..."
            post:
              - echo "Packaging complete!"
              - ls -la dist/
```

### Specification

Hooks are defined under `package.hooks` in a job. The value of each hook can be:

| Type     | Example                                              |
| -------- | ---------------------------------------------------- |
| `string` | `pre: echo "before"`                                 |
| `list`   | `post: [echo "step 1", echo "step 2"]`            |

| Hook  | Description                    | Fail on error |
| ----- | ------------------------------ | ------------- |
| `pre` | Runs before packaging starts   | ✅ Yes        |
| `post`| Runs after packaging completes | ✅ Yes        |

> **Note**: If a hook command exits with a non-zero exit code, the packaging process is aborted.

### Environment variables

The following environment variables are available in hook commands:

| Variable                | Description                                            |
| ----------------------- | ------------------------------------------------------ |
| `PLATFORM`              | Platform name, e.g. `android`, `ios`, `macos`          |
| `PACKAGE_FORMAT`        | Package format, e.g. `apk`, `ipa`, `dmg`               |
| `BUILD_MODE`            | Build mode, e.g. `release`, `profile`                  |
| `OUTPUT_DIRECTORY`      | Output directory path                                  |
| `BUILD_OUTPUT_DIRECTORY`| Flutter build output directory path                    |
| `BUILD_OUTPUT_FILES`    | Colon-separated list of build output file paths        |
