# fir.im

English | [简体中文](../../zh-Hans/publishers/fir.md)

The `fir` target uploads an Android APK or iOS IPA to fir.im.

## Configuration

```bash
export FIR_API_TOKEN=fir-api-token
```

## Publish

```bash
fastforge publish --path dist/app.apk --target fir
```

Fastforge reads the bundle ID, app name, version, and build number from the APK or IPA. The publishing result is the fir.im download URL for the new release.

## Optional Arguments

Explicit arguments override the values read from the package:

| Argument       | Description      |
| -------------- | ---------------- |
| `bundle_id`    | Bundle ID / application ID |
| `app_name`     | App display name |
| `version`      | Version name     |
| `build_number` | Build number     |

`changelog` sets the release's update notes, for example `--publish-arg changelog="Fixes crash on launch"`.

Hyphenated forms (`bundle-id`, `app-name`, `build-number`) are also accepted. If the package cannot be parsed, the upload continues only when `bundle_id` is provided.

The platform is inferred only from the `.apk` or `.ipa` extension; other extensions fail.
