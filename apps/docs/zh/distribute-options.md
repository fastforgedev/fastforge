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

## Hooks

Package hooks 允许你在 job 的打包步骤前后执行 shell 命令。

### 示例

```yaml
releases:
  - name: dev
    jobs:
      - name: android-apk
        package:
          platform: android
          target: apk
          hooks:
            pre: echo "开始打包..."
            post:
              - echo "打包完成!"
              - ls -la dist/
```

### 规范

Hooks 定义在 job 的 `package.hooks` 下。每个 hook 的值可以是：

| 类型     | 示例                                                |
| -------- | ---------------------------------------------------- |
| `string` | `pre: echo "before"`                                 |
| `list`   | `post: [echo "step 1", echo "step 2"]`            |

| Hook   | 描述               | 失败时终止 |
| ------ | ------------------ | ---------- |
| `pre`  | 打包前执行         | ✅ 是      |
| `post` | 打包完成后执行     | ✅ 是      |

> **注意**：如果 hook 命令返回非零退出码，打包过程将被终止。

### 环境变量

以下环境变量可在 hook 命令中使用：

| 变量                     | 说明                                            |
| ------------------------ | ----------------------------------------------- |
| `PLATFORM`               | 平台名称，如 `android`、`ios`、`macos`          |
| `PACKAGE_FORMAT`         | 包格式，如 `apk`、`ipa`、`dmg`                  |
| `BUILD_MODE`             | 构建模式，如 `release`、`profile`               |
| `OUTPUT_DIRECTORY`       | 输出目录路径                                    |
| `BUILD_OUTPUT_DIRECTORY` | Flutter 构建产物输出目录路径                    |
| `BUILD_OUTPUT_FILES`     | 冒号分隔的构建产物文件路径列表                  |
