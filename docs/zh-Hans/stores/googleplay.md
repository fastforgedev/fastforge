# Google Play

[English](../../en/stores/googleplay.md) | 简体中文

`fastforge googleplay` 直接操作 Google Play Developer API，覆盖应用校验、edit、AAB 上传、track 和 catalog。

## 认证

`GOOGLE_PLAY_SERVICE_ACCOUNT_JSON` 可以是完整服务账号 JSON 或文件路径：

```bash
export GOOGLE_PLAY_SERVICE_ACCOUNT_JSON="$PWD/service-account.json"
```

服务账号需要目标应用的 Google Play Developer API 权限。

## 应用

```bash
fastforge googleplay app view com.example.myapp
fastforge googleplay app check com.example.myapp
```

`app view` 只输出 package name 和 Play Console 链接，不调用 API。`app check` 通过创建并删除一个 edit 来校验访问权限。

## Edit 工作流

大多数写操作发生在 edit 中：

```bash
fastforge googleplay edit create \
  --package-name com.example.myapp

fastforge googleplay edit commit \
  --package-name com.example.myapp \
  --edit-id <edit-id>
```

不再需要 edit 时可以使用 `edit delete`。

## 上传 AAB

复用现有 edit：

```bash
fastforge googleplay bundle upload dist/app-release.aab \
  --package-name com.example.myapp \
  --edit-id <edit-id>
```

也可以指定 track 并在上传后直接提交：

```bash
fastforge googleplay bundle upload dist/app-release.aab \
  --package-name com.example.myapp \
  --track internal \
  --release-name '1.0.0 (1)' \
  --commit
```

- 只接受 `.aab` 文件。
- 未指定 `--edit-id` 时会新建 edit；未指定 `--commit` 时该 edit 保持未提交状态，输出中会包含 edit ID。
- 未指定 `--track` 时只上传 bundle，不会分配到任何 track。
- `--status` 默认 `completed`；`--release-name` 默认使用去掉 `.aab` 扩展名的 AAB 文件名。

## Track

```bash
fastforge googleplay track list \
  --package-name com.example.myapp \
  --edit-id <edit-id>

fastforge googleplay track view internal \
  --package-name com.example.myapp \
  --edit-id <edit-id>

fastforge googleplay track update internal \
  --package-name com.example.myapp \
  --edit-id <edit-id> \
  --version-code 1 \
  --status completed

fastforge googleplay edit commit \
  --package-name com.example.myapp \
  --edit-id <edit-id>
```

Track 名称包括 `internal`、`alpha`、`beta`、`production` 以及自定义 track。`track update` 不会提交 edit，之后务必执行 `edit commit`。

`track update` 与 `bundle upload --track` 会用单个 release 替换该 track 的全部 release，其中只包含 release 名称、version code 和状态。`track update` 的 `--release-name` 默认为 `release <version-code>`。命令没有设置发布比例（`userFraction`）或更新说明的选项，分阶段发布和更新说明需要使用 `fastforge googleplay api`，或通过 [catalog](catalog.md) 推送 track YAML 文件。

## Catalog 与原始 API

- Listing、图片和 track 元数据同步见[统一 Catalog](catalog.md)。
- 未封装接口可以通过 `fastforge googleplay api` 调用：

```bash
fastforge googleplay api get /applications/com.example.myapp/edits/<edit-id>/tracks
fastforge googleplay api put /applications/com.example.myapp/edits/<edit-id>/tracks/beta \
  --input track.json
```

支持的方法为 `get`、`post`、`put`、`patch` 和 `delete`。路径相对于 `androidpublisher/v3`，且必须以 `/applications/` 开头；`--query KEY=VALUE` 可重复传入，`post`/`put`/`patch` 可用 `--input` 指定 JSON 请求体文件。
