# 统一 Catalog

[English](../../en/stores/catalog.md) | 简体中文

Catalog 把 App Store 与 Google Play 的元数据和图片同步到本地目录，便于版本控制、批量编辑和重复发布。AppGallery 暂不支持 catalog 同步。

## 批量同步

先在 `.fastforge/config.yaml` 登记应用，然后运行：

```bash
fastforge store catalog pull
fastforge store catalog push
```

- 命令先按配置顺序处理全部 App Store 应用，再处理全部 Google Play 应用。AppGallery 应用会出现在 `fastforge store list` 中，但 catalog 命令会跳过它们。
- App Store 条目使用 `bundle_id`（缺失时回退到 `app_id`）和 `apps[].platform`；未配置 `platform` 的旧配置默认使用 `IOS`。
- 聚合命令始终使用默认目录，且 `fastforge store catalog push` 没有 `--dry-run`，会直接推送。
- 单个应用失败不会中断后续应用；最终只要存在失败，进程就以错误状态退出。
- bundle ID 相同但平台不同的两个 App Store 条目共用同一目录。每次 pull 都会替换该目录，因此只会保留最后一个条目的平台。这类平台请使用不同的 `--output` 根目录分别拉取。

## 默认目录

```text
.fastforge/stores/
├── appstore/
│   └── com.example.myapp/
└── googleplay/
    └── com.example.myapp/
```

## 单独同步 App Store

```bash
fastforge appstore catalog pull \
  --app com.example.myapp \
  --platform MAC_OS \
  --version 1.0.0 \
  --output .fastforge/stores/appstore

fastforge appstore catalog push \
  --app com.example.myapp \
  --input .fastforge/stores/appstore \
  --dry-run
```

`--app` 接受与 `appstore app view` 相同的标识。`--output` 和 `--input` 是根目录，命令会在其后追加解析出的 bundle ID 目录。`--platform` 默认 `IOS`；省略 `--version` 时拉取该平台的全部版本。

Catalog pull 会先在目标目录旁生成完整快照，所有必要请求和写入成功后才替换整个 `<bundle-id>/` 目录。pull 失败时会丢弃 staging 文件并保留上一次快照。pull 成功时，新快照之外的内容都会被移除，包括其他平台、其他版本以及未提交的本地修改。单个截图或 preview 下载失败只会输出警告并缺少该文件；尚无下载地址的媒体会写成空的占位文件。push 前请检查这两种情况。若使用尚无此行为的旧版或 unofficial 二进制，失败可能留下部分文件：不要执行 push；先确认版本目录以及存在截图时的 `.manifest.yaml` 完整，或重新拉取到新目录。

目录结构：

```text
<bundle-id>/
├── app.yaml
├── app_info.yaml
├── info/
│   └── en-US.yaml
├── versions/
│   └── IOS/
│       └── 1.0.0/
│           ├── version.yaml
│           ├── review.yaml
│           ├── review.d/
│           │   └── <attachment-id>.yaml
│           └── en-US/
│               ├── localization.yaml
│               ├── screenshots/
│               │   └── <DISPLAY_TYPE>/001.png
│               └── previews/
│                   └── <PREVIEW_TYPE>/001_<id>.mov
└── .manifest.yaml
```

- `app.yaml` 记录 bundle ID、名称、主语言和 SKU，不会被 push。
- `app_info.yaml` 管理主分类、次分类和子分类。
- `info/<locale>.yaml` 保存应用信息本地化字段，例如名称、副标题和隐私政策 URL。
- `version.yaml` 保存版本级字段（`_id`、`platform`、`versionString`、`state`、`copyright`）。
- `review.yaml` 保存 App Review 审核信息；`review.d/` 保存审核附件元数据。
- `localization.yaml` 保存语言相关字段，例如描述、关键词、更新说明、推广文本和 URL。
- `.manifest.yaml` 保存每个截图集的远端截图 ID 与校验和。

pull 会在版本之间去重：每个平台按从旧到新的顺序处理版本，本地化字段、版权、`review.yaml` 以及截图或 preview 集只有与上一个版本不同时才会写入。版本目录内容稀疏属于正常情况。

push 行为：

- 更新 `app_info.yaml` 与 `info/<locale>.yaml`，并创建缺失的语言；不会删除远端语言。
- `version.yaml` 只推送 `copyright`。远端不存在对应版本的目录会被跳过；push 不会创建版本。
- `localization.yaml` 会更新有变化的字段并创建缺失的语言；不会删除远端语言。
- 截图按包含文件的 display type 目录同步（仅支持 PNG 或 JPEG，其他非隐藏文件会导致 push 失败），缺失的截图集会被创建：未变化的文件按校验和复用，新文件上传，本地没有对应文件的远端截图会被删除，顺序按本地文件名排列。空目录或缺失目录不会改动对应的远端截图集。
- preview 只拉取，不推送。
- `review.yaml` 会被更新或创建。`review.d/` 只更新已有附件，不会上传新附件。

## 单独同步 Google Play

```bash
fastforge googleplay catalog pull \
  --package-name com.example.myapp \
  --output .fastforge/stores/googleplay

fastforge googleplay catalog push \
  --package-name com.example.myapp \
  --input .fastforge/stores/googleplay \
  --dry-run
```

`--output` 和 `--input` 是根目录，命令会在其后追加 package name 目录。

目录结构：

```text
<package-name>/
├── app.yaml
├── listings/
│   └── <language>.yaml
├── screenshots/
│   └── <language>/
│       └── <type>/NNN_<image-id>.<ext>
└── tracks/
    └── <track>.yaml
```

- `<type>` 为 `phone_screenshots`、`seven_inch_screenshots`、`ten_inch_screenshots`、`tv_screenshots`、`wear_screenshots`、`feature_graphic`、`tv_banner` 或 `icon` 之一。
- track 名称中的冒号在文件名中会替换为下划线（例如 `wear:production` 对应 `wear_production.yaml`）。
- Google Play pull 会写入已有目录且从不删除文件。远端已删除的图片仍会保留在本地，并会在 push 时重新上传；需要精确快照时请拉取到空目录。

push 在单个 edit 中执行，结束时自动提交：

- `listings/*.yaml` 会被创建或更新；不会删除远端语言。
- 对每个包含文件的图片类型目录，先删除该语言和类型下的全部远端图片，再按文件名顺序上传本地文件。单张图片上传失败只会输出警告，图片集可能因此不完整。空目录或缺失目录不会改动远端图片。
- 每个 `tracks/*.yaml` 文件都会原样发送，并替换该 track 的 release，包括状态、`userFraction` 和更新说明。过期的 track 文件可能改动线上 release；push 前请移除不打算修改的 track 文件。

## 安全操作顺序

1. `pull` 获取最新远端状态。App Store 需拉取所需的每个平台和版本，会共用目录时使用不同根目录；Google Play 请拉取到空目录。
2. 在独立分支编辑 YAML 和图片。
3. 校验 diff。从非空目录中删除截图会同时删除远端截图；删除整个语言或图片目录不会删除任何远端内容。
4. 使用独立商店命令的 `push --dry-run` 预览。dry run 只列出每个文件的计划操作，不是字段级 diff；App Store 会把 `info/` 下每个语言以及每个版本的 `localization.yaml` 都列为 `create`，即使远端已存在。
5. 确认后执行实际 push。
