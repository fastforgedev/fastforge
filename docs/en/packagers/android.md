# Android

English | [简体中文](../../zh-Hans/packagers/android.md)

Fastforge builds and prepares Android application artifacts, supporting [APK](#apk) and [AAB](#aab).

## Current Status

| Build system    | Project type            | `package` status      |
| --------------- | ----------------------- | --------------------- |
| Gradle          | No `pubspec.yaml`       | APK and AAB supported |
| Flutter Builder | Contains `pubspec.yaml` | APK and AAB supported |

The packager copies the built APK or AAB to `dist/<version>/` with the configured artifact name. Android builds are not reused across targets: `--targets apk,aab` runs two builds. Native Gradle projects have additional limitations; see [Gradle Builder](../builders/gradle.md#limitations).

## Requirements

- Android SDK
- A working Gradle and Android toolchain (plus the Flutter SDK for Flutter projects)
- For APK/AAB analysis, configure `ANDROID_HOME` and `aapt2`

## APK

An APK is an Android application package that can be installed directly:

```bash
fastforge package --targets apk
```

In a Flutter project, Flutter build options apply, for example:

```bash
fastforge package --targets apk \
  --build-flavor dev \
  --build-dart-define APP_ENV=dev
```

Workflow example:

```yaml
- name: Package APK
  uses: fastforge/package
  with:
    platform: android
    target: apk
    output: artifacts/
```

With `--flutter-build-args split-per-abi`, every split APK is packaged, with its ABI appended to the artifact name, such as `<artifact>-arm64-v8a.apk`. Without it, only the universal APK is packaged, even when the build directory still holds split APKs from an earlier build.

To generate only the raw Flutter artifact, run `fastforge build --platform android --target apk`. See [Flutter Builder](../builders/flutter.md) for all options.

## AAB

An AAB (Android App Bundle) is used for Google Play distribution:

```bash
fastforge package --targets aab
```

To generate only the raw Flutter artifact, run `fastforge build --platform android --target aab`.

### Upload to Google Play

For a direct upload of an AAB to a track, use the `playstore` publisher (it reads a service account key from `PLAYSTORE_CREDENTIALS`):

```bash
fastforge publish --path dist/<version>/<artifact>.aab --targets playstore \
  --playstore-package-name com.example.app --playstore-track internal
```

For edits, track updates, and rollouts, use the Google Play commands:

```bash
fastforge googleplay bundle upload --help
fastforge googleplay track update --help
```

See the [Play Store publisher](../publishers/playstore.md) and [Google Play](../stores/googleplay.md) for authentication and command details.

Return to the [packager overview](README.md).
