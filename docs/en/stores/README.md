# Store Management

English | [简体中文](../../zh-Hans/stores/README.md)

Store commands have a different responsibility from the general `publish` command. Publishers upload one artifact, while store commands query apps, manage versions and tracks, submit reviews, and synchronize metadata and images.

## Entry Points

| Entry point            | Purpose                                   | Documentation                    |
| ---------------------- | ----------------------------------------- | -------------------------------- |
| `fastforge appstore`   | App Store Connect API, builds, and review | [App Store Connect](appstore.md) |
| `fastforge appgallery` | Huawei apps, packages, and releases       | [AppGallery Connect](appgallery.md) |
| `fastforge googleplay` | Google Play edits, AABs, and tracks       | [Google Play](googleplay.md)     |
| `fastforge store`      | List configured apps; App Store and Google Play catalogs | [Unified Catalog](catalog.md)    |

## Project Configuration

The store application list lives in `.fastforge/config.yaml`:

```yaml
stores:
  appstore:
    apps:
      - bundle_id: com.example.myapp
        app_id: "1234567890"
  appgallery:
    apps:
      - app_id: "987654321"
        package_name: com.example.myapp
  googleplay:
    apps:
      - package_name: com.example.myapp
        track: production
```

```bash
fastforge store list
```

Provide authentication credentials through process environment variables. See [Store Configuration](configuration.md) for all fields.

## Output Options

The `appstore`, `appgallery`, and `googleplay` commands support:

- `--json <FIELDS>`
- `--verbose`
- `--debug`
- `--no-color`

`--json` takes a comma-separated list of fields and prints the selected fields of typed query results as JSON (an unknown field fails and lists the available ones); raw `api`, `catalog`, and `appgallery release` commands ignore it. `--verbose`, `--debug`, and `--no-color` are accepted but currently have no effect.

`appstore` and `googleplay` also accept `--limit <LIMIT>`. In `appstore` it is passed to `app list`, `build list`, `version list`, `submission list`, and `submission items`; in `googleplay` it currently has no effect. For `appgallery`, only `package list` has its own `--limit` and `--offset`. `appstore` accepts `--paginate`, but it currently has no effect.
