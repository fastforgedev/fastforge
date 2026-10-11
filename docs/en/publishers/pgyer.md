# PGYER

English | [简体中文](../../zh-Hans/publishers/pgyer.md)

The `pgyer` target uploads an app package to PGYER (蒲公英).

## Configuration

```bash
export PGYER_API_KEY=pgyer-api-key
```

## Publish

```bash
fastforge publish --path dist/app.apk --target pgyer
```

The file extension (for example `apk` or `ipa`) is sent as PGYER's build type, so keep the extension accurate. After the upload, Fastforge polls the build information every 3 seconds, retrying up to 10 times while PGYER is still processing the build. The publishing result is `http://www.pgyer.com/<build key>`.

## Optional Arguments

Each argument is available as a `--pgyer-<name>` option or a `--publish-arg`:

| Argument             | Description                                            |
| -------------------- | ------------------------------------------------------ |
| `oversea`            | Upload acceleration: `1` overseas, `2` domestic         |
| `install-type`       | `1` public, `2` password, `3` invitation               |
| `password`           | Install password (for password installation)           |
| `description`        | App description                                        |
| `update-description` | Update description for this version                    |
| `install-date`       | Install validity: `1` time range, `2` permanent         |
| `install-start-date` | Validity start date, e.g. `2018-01-01`                  |
| `install-end-date`   | Validity end date, e.g. `2018-12-31`                    |
| `channel-shortcut`   | Channel shortcut to update                             |

Numeric arguments with non-numeric values are ignored.

```bash
fastforge publish --path dist/app.apk --target pgyer \
  --pgyer-install-type 2 \
  --pgyer-password 1234 \
  --pgyer-update-description 'Internal build'
```
