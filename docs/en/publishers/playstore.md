# Google Play

English | [简体中文](../../zh-Hans/publishers/playstore.md)

The `playstore` target uploads an Android App Bundle to Google Play and can assign it to a track.

## Authentication

`PLAYSTORE_CREDENTIALS` must be the **path** to a service account JSON file with Google Play Developer API access to the app:

```bash
export PLAYSTORE_CREDENTIALS="$PWD/service-account.json"
```

## Publish

Only `.aab` files are accepted:

```bash
fastforge publish --path dist/app-release.aab --target playstore \
  --publish-arg package-name=com.example.app \
  --publish-arg track=internal
```

| Argument           | Required | Description                                                     |
| ------------------ | :------: | --------------------------------------------------------------- |
| `package-name`     |   Yes    | Application package name (`--playstore-package-name`)           |
| `credentials-file` |    No    | Overrides `PLAYSTORE_CREDENTIALS`                               |
| `track`            |    No    | Track to assign the uploaded version code to (`--playstore-track`): `production`, `beta` (open testing), `alpha` (closed testing), `internal`, or a custom track name |

The publisher creates an edit, uploads the bundle, and commits the edit. When `track` is set, it first updates that track with a release containing the new version code and status `completed` (a full rollout). The release name is `<build> (<version>)` when the second `-`-separated segment of the file name is `<version>+<build>` (for example `my_app-1.0.0+5-android.aab`); otherwise it is the app version, or `release`. The publishing result is the Play Console URL for the app.

## Subsequent Management

Track inspection and store metadata are handled by `fastforge googleplay`; staged rollouts and release notes require `fastforge googleplay api` or track files pushed through the catalog. See [Google Play](../stores/googleplay.md).
