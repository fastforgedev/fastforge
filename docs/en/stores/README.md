# Store Management

English | [简体中文](../../zh-Hans/stores/README.md)

Store commands have a different responsibility from the general `publish` command. Publishers upload one artifact, while store commands query apps, manage versions and tracks, submit reviews, and synchronize metadata and images.

## Entry Points

| Entry point            | Purpose                                   | Documentation                    |
| ---------------------- | ----------------------------------------- | -------------------------------- |
| `fastforge appstore`   | App Store Connect API, builds, and review | [App Store Connect](appstore.md) |
| `fastforge appgallery` | Huawei apps, packages, and releases       | [AppGallery Connect](appgallery.md) |
| `fastforge googleplay` | Google Play edits, AABs, and tracks       | [Google Play](googleplay.md)     |
| `fastforge store`      | Process multiple configured store apps    | [Unified Catalog](catalog.md)    |

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
- `--limit <LIMIT>`
- `--verbose`
- `--debug`
- `--no-color`

App Store commands additionally support `--paginate`.
