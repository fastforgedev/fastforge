# Android

[English](../../en/packagers/android.md) | 简体中文

Fastforge 构建并整理 Android 应用产物，支持 [APK](#apk) 和 [AAB](#aab) 两种格式。

## 当前状态

| 构建系统        | 适用项目            | `package` 状态  |
| --------------- | ------------------- | --------------- |
| Gradle          | 不含 `pubspec.yaml` | APK、AAB 已支持 |
| Flutter Builder | 含 `pubspec.yaml`   | APK、AAB 已支持 |

打包器会按配置的产物名把构建出的 APK 或 AAB 复制到 `dist/<version>/`。Android 构建不会在多个 target 间复用：`--targets apk,aab` 会执行两次构建。原生 Gradle 项目还有额外限制，见 [Gradle Builder](../builders/gradle.md#限制)。

## 环境要求

- Android SDK
- 可用的 Gradle 和 Android 工具链（Flutter 项目还需要 Flutter SDK）
- 需要分析 APK/AAB 时，配置 `ANDROID_HOME` 和 `aapt2`

## APK

APK 是 Android 可直接安装的应用包：

```bash
fastforge package --targets apk
```

Flutter 项目可以使用 Flutter 构建参数，例如：

```bash
fastforge package --targets apk \
  --build-flavor dev \
  --build-dart-define APP_ENV=dev
```

工作流示例：

```yaml
- name: Package APK
  uses: fastforge/package
  with:
    platform: android
    target: apk
    output: artifacts/
```

使用 `--flutter-build-args split-per-abi` 时，会打包每个按 ABI 拆分的 APK，并在产物名后追加 ABI，例如 `<artifact>-arm64-v8a.apk`。未使用该参数时只打包通用 APK，即使构建目录里还留有之前构建的拆分 APK。

只需要 Flutter 原始产物时，运行 `fastforge build --platform android --target apk`。完整参数见 [Flutter Builder](../builders/flutter.md)。

## AAB

AAB（Android App Bundle）用于 Google Play 分发：

```bash
fastforge package --targets aab
```

只需要 Flutter 原始产物时，运行 `fastforge build --platform android --target aab`。

### 上传 Google Play

直接把 AAB 上传到某个轨道时，使用 `playstore` 发布器（从 `PLAYSTORE_CREDENTIALS` 读取服务账号密钥）：

```bash
fastforge publish --path dist/<version>/<artifact>.aab --targets playstore \
  --playstore-package-name com.example.app --playstore-track internal
```

需要管理 edit、轨道更新和分阶段发布时，使用 Google Play 命令：

```bash
fastforge googleplay bundle upload --help
fastforge googleplay track update --help
```

认证和命令说明见 [Play Store 发布器](../publishers/playstore.md)与 [Google Play](../stores/googleplay.md)。

返回[打包器总览](README.md)。
