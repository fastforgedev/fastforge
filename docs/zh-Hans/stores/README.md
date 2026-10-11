# 商店管理

[English](../../en/stores/README.md) | 简体中文

商店命令与通用 `publish` 的职责不同：发布器负责上传单个产物，商店命令负责查询应用、管理版本与轨道、提交审核，以及同步元数据和图片。

## 入口

| 入口                   | 用途                              | 文档                             |
| ---------------------- | --------------------------------- | -------------------------------- |
| `fastforge appstore`   | App Store Connect API、构建与审核 | [App Store Connect](appstore.md) |
| `fastforge appgallery` | 华为应用、软件包与提交审核        | [AppGallery Connect](appgallery.md) |
| `fastforge googleplay` | Google Play edit、AAB 与 track    | [Google Play](googleplay.md)     |
| `fastforge store`      | 列出配置的应用；App Store 与 Google Play catalog | [统一 Catalog](catalog.md)       |

## 项目配置

商店应用清单位于 `.fastforge/config.yaml`：

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

认证凭证应通过进程环境变量提供，完整字段见[商店配置](configuration.md)。

## 输出选项

`appstore`、`appgallery` 与 `googleplay` 命令支持：

- `--json <FIELDS>`
- `--verbose`
- `--debug`
- `--no-color`

`--json` 接受逗号分隔的字段列表，以 JSON 输出类型化查询结果中选定的字段（字段不存在时会报错并列出可用字段）；原始 `api`、`catalog` 与 `appgallery release` 命令会忽略该选项。`--verbose`、`--debug` 与 `--no-color` 可被接受，但目前不起作用。

`appstore` 与 `googleplay` 还支持 `--limit <LIMIT>`。`appstore` 会把它传给 `app list`、`build list`、`version list`、`submission list` 和 `submission items`；`googleplay` 目前不使用该选项。`appgallery` 只有 `package list` 自带 `--limit` 和 `--offset`。`appstore` 接受 `--paginate`，但该选项目前不起作用。
