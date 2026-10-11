# App Store Connect

[English](../../en/stores/appstore.md) | 简体中文

`fastforge appstore` 直接调用 App Store Connect API，覆盖应用查询、构建上传、版本提交、审核 submission 和 catalog。

支持的平台为 `IOS`、`MAC_OS`、`TV_OS` 和 `VISION_OS`。带 `--platform` 选项的命令均接受这些值；`catalog pull` 默认使用 `IOS`。

## 认证

```bash
export APP_STORE_CONNECT_KEY_ID=ABC123DEFG
export APP_STORE_CONNECT_ISSUER_ID=00000000-0000-0000-0000-000000000000
export APP_STORE_CONNECT_KEY_PATH="$PWD/AuthKey_ABC123DEFG.p8"
```

三个变量均为必填。商店 API 命令只使用 API Key 认证。

使用 `direnv` 的项目要注意：非交互 shell、CI 和 Agent 工具不会执行交互 shell hook。只检查变量是否已设置，不要输出凭证值；后续所有 App Store 命令都保持在同一环境中运行：

```bash
direnv status
direnv exec . fastforge appstore app view com.example.myapp
```

`direnv exec .` 也会发现父目录中的 `.envrc`。这样可避免误用全局凭证而查询到另一个 App Store Connect 团队。

## 应用

```bash
fastforge appstore app list
fastforge appstore app view com.example.myapp
```

`view` 依次尝试按数字 App Store app id、bundle id、SKU 和应用名称匹配；匹配到多个应用时会报错。下文所有 `--app` 选项都使用相同的解析方式。

“建立 Store 档案”可能指三种不同操作：

1. 在 `.fastforge/config.yaml` 本地登记已有应用，供 `fastforge store` 聚合命令使用。
2. 对已有远端应用执行 `catalog pull`，建立本地元数据快照。
3. 在 App Store Connect 新建远端应用记录。本地登记和 `catalog pull` 都不会创建远端应用。

## 构建

```bash
fastforge appstore build upload dist/MyApp.ipa \
  --app com.example.myapp \
  --wait

fastforge appstore build upload dist/MyApp.pkg \
  --app com.example.myapp

fastforge appstore build list --app com.example.myapp --version 1.0.0
fastforge appstore build view <build-id>
fastforge appstore build wait <build-id> --timeout 30m
```

- 上传通过 `xcrun altool` 执行，因此需要 macOS。`.pkg` 文件按 macOS 应用上传，其他文件按 iOS 应用上传。
- `build wait` 每 30 秒轮询一次，直到处理状态为 `VALID`；遇到 `FAILED`、`INVALID` 或超时则失败。`--timeout` 默认 `30m`，支持 `m` 或 `s` 后缀；不带后缀的数字按秒计算。
- `build upload --wait` 使用固定的 30 分钟超时，并等待该应用最近上传的构建。如果新构建尚未出现在列表中，可能会选中旧构建；需要确定时，请先 `build list`，再执行 `build wait <build-id>`。
- Fastforge 没有 TestFlight 命令（beta 群组、测试员、beta 审核），请使用 App Store Connect 或原始 API。

## 版本

```bash
fastforge appstore version list --app com.example.myapp
fastforge appstore version view 1.0.0 --app com.example.myapp
fastforge appstore version submit 1.0.0 \
  --app com.example.myapp \
  --build <build-id> \
  --wait
```

`version view` 与 `version submit` 接受版本号或数字 App Store version ID。

`version submit` 会关联构建、按版本所属平台创建审核 submission、添加版本 item 并提交审核。

- 版本必须已在 App Store Connect 中存在；Fastforge 没有创建版本的命令。
- `--build latest` 会选择该版本号下最新的构建。
- 命令不会检查构建处理状态，请先等待构建处理完成。
- `--wait` 使用固定的 30 分钟超时，submission 进入 `WAITING_FOR_REVIEW`、`IN_REVIEW`、`COMPLETING` 或 `COMPLETE` 后即返回，不会等待 App Review 审核通过。

## 审核 submission

```bash
fastforge appstore submission list --app com.example.myapp
fastforge appstore submission view <submission-id>
fastforge appstore submission create \
  --app com.example.myapp \
  --platform IOS
fastforge appstore submission items <submission-id>
fastforge appstore submission add-item <submission-id> \
  --item-type appStoreVersions \
  --item-id <version-id>
fastforge appstore submission remove-item <item-id>
fastforge appstore submission submit <submission-id> --wait --timeout 30m
fastforge appstore submission cancel <submission-id>
```

- `list` 可以按 `--platform` 和 `--state` 筛选。状态包括 `READY_FOR_REVIEW`、`WAITING_FOR_REVIEW`、`IN_REVIEW`、`UNRESOLVED_ISSUES`、`CANCELING`、`COMPLETING`、`COMPLETE`。
- `create` 未指定 `--platform` 时不传平台，由 App Store Connect 决定。
- `remove-item` 接受 `items` 列出的 submission item ID。
- `submit --wait` 每 30 秒轮询一次，submission 进入 `WAITING_FOR_REVIEW`、`IN_REVIEW`、`COMPLETING` 或 `COMPLETE` 后返回；遇到 `UNRESOLVED_ISSUES`、`CANCELING` 或超时（`--timeout`，默认 `30m`）则失败。
- `add-item --item-type` 支持 `appStoreVersions`、`appCustomProductPageVersions`、`appStoreVersionExperiments`、`appStoreVersionExperimentsV2`、`appEvents`、`backgroundAssetVersions`、`gameCenterAchievementVersions`、`gameCenterActivityVersions`、`gameCenterChallengeVersions`、`gameCenterLeaderboardSetVersions` 和 `gameCenterLeaderboardVersions`。

## Catalog

App Store 的元数据、分类、截图和 preview 见[统一 Catalog](catalog.md)。

## 原始 API

```bash
fastforge appstore api get /v1/apps --query 'filter[bundleId]=com.example.myapp'
fastforge appstore api patch /v1/appStoreVersions/<version-id> --input version.json
```

支持的方法为 `get`、`post`、`patch` 和 `delete`。路径必须以 `/v1/` 开头，`--query KEY=VALUE` 可重复传入，`post`/`patch` 可用 `--input` 指定 JSON 请求体文件。响应会格式化为 JSON 输出。

原始 API 适合尚未封装的 App Store Connect 资源；自动化脚本应优先使用已有的类型化命令。
