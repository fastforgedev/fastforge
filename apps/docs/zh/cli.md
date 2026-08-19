# CLI

如何使用 Fastforge 的命令行界面（CLI）

## 安装

```shell
dart pub global activate fastforge
```

> **Windows 用户请注意：** 激活后，请确保 pub 缓存 bin 目录已添加到 PATH 环境变量中：
> 1. 打开 **系统属性** → **高级** → **环境变量**
> 2. 在 **用户变量** 中找到 `Path`，点击 **编辑**
> 3. 添加 `%APPDATA%\Pub\Cache\bin` 并点击 **确定**
> 4. 重启终端，然后运行 `fastforge --help` 验证

---

## 命令

> 这些命令按字母顺序排列，最常用的是 package、publish 和 release。

### Package

将应用程序打包为特定于平台的格式，并将结果放入文件夹中。

| Flag | Value | Required |
|------|-------|:--------:|
| `--platform` | 平台, 如 `android`（省略时根据 targets 与项目结构自动推导） | ❌ |
| `--targets` | 以逗号分隔的 maker 名称列表 | ✅ |
| `--skip-clean` | 跳过构建前的清理 | ❌ |
| `--hook-pre` | 打包前执行的 shell 命令 | ❌ |
| `--hook-post` | 打包后执行的 shell 命令 | ❌ |

示例：

```shell
fastforge package --platform=android --targets=aab,apk

# 平台可省略：`aab`/`apk` 只属于 android，会自动推导
fastforge package --targets=aab,apk

fastforge package --platform=macos --target=zip --hook-pre 'echo "before"' --hook-post 'echo "after"'
```

> `--platform` 可省略。大多数 target 能唯一确定平台（如 `dmg` → macos、`apk` → android）；`zip`、`direct`、`custom` 等有歧义的 target 会结合项目中的平台目录与当前系统推导，无法唯一确定时才需要显式指定。

### Publish

| Flag | Value | Required |
|------|-------|:--------:|
| `--path` | 路径, 如 `hello_world-1.0.0+1-android.apk` | ✅ |
| `--targets` | 以逗号分隔的 publisher 名称列表 | ✅ |

示例：

```shell
fastforge publish --path hello_world-1.0.0+1-android.apk --targets fir,pgyer
```

### Release

会根据配置文件（`distribute_options.yaml`），将你的应用打包成特定的格式并发布到分发平台。

| Flag | Value | Required |
|------|-------|:--------:|
| `--name` | 名称, e.g. `dev` | ✅ |
| `--skip-clean` | 跳过构建前的清理 | ❌ |

示例：

```shell
fastforge release --name dev
```

### App Store 审核提交

`appstore submission` 命令用于管理 App Store Connect 当前的审核提交流程。
认证使用 `APP_STORE_CONNECT_KEY_ID`、`APP_STORE_CONNECT_ISSUER_ID` 和
`APP_STORE_CONNECT_KEY_PATH`。

```shell
# 查看 submission 及其 item
fastforge appstore submission list --app com.example.myapp
fastforge appstore submission view <submission-id>
fastforge appstore submission items <submission-id>

# 创建草稿、添加 App Store 版本并提交审核
fastforge appstore submission create --app com.example.myapp --platform IOS
fastforge appstore submission add-item <submission-id> \
  --item-type appStoreVersions --item-id <version-id>
fastforge appstore submission submit <submission-id> --wait

# 删除草稿 item，或取消已经提交的审核
fastforge appstore submission remove-item <submission-item-id>
fastforge appstore submission cancel <submission-id>
```

`list` 支持可选的 `--platform` 与 `--state` 筛选。`add-item` 还支持自定义产品页版本、
版本实验、App 内活动、后台资源及 Game Center 版本等 App Store Connect 可审核资源类型。
读取和修改命令均可配合 `--json` 输出结构化数据。

`fastforge appstore version submit <version> --app <app> --build <build>` 也会自动使用
这套审核提交流程：关联构建、创建 submission、添加 App Store 版本 item，最后提交审核。

### Store catalog

通过 `.fastforge/config.yaml` 配置需要同步的 App Store 和 Google Play 应用：

```yaml
stores:
  appstore:
    apps:
      - bundle_id: com.example.myapp
        app_id: "1234567890" # 省略 bundle_id 时的可选回退值
  googleplay:
    apps:
      - package_name: com.example.myapp
```

然后使用统一命令拉取或推送所有已配置应用的 catalog 数据：

```shell
fastforge store catalog pull
fastforge store catalog push
```

统一命令从配置文件读取应用标识，认证继续使用现有环境变量：

| 商店 | 环境变量 |
|------|----------|
| App Store | `APP_STORE_CONNECT_KEY_ID`、`APP_STORE_CONNECT_ISSUER_ID`、`APP_STORE_CONNECT_KEY_PATH` |
| Google Play | `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON`（服务账号 JSON 内容或 JSON 文件路径） |

Catalog 文件仍使用 `.fastforge/stores/appstore/` 和
`.fastforge/stores/googleplay/` 下的默认目录。命令会按配置顺序处理全部应用；
单个应用失败不会中断后续应用，最终会打印汇总并以错误状态退出。

对于 App Store 版本，`pull` 会把 `copyright` 等版本级属性写入
`versions/<platform>/<version>/version.yaml`，并把本地化属性写入
`versions/<platform>/<version>/<locale>/localization.yaml`。修改这些文件后运行
`push`，即可更新 App Store Connect 中对应的资源。同一平台的连续版本若
copyright 未变化，则不会重复创建 `version.yaml`。`push` 仍兼容语言目录下旧的
`version.yaml`；若同一语言目录同时存在两个文件名，则会报错以避免歧义。

App Store 的应用级分类会写入 `<bundle-id>/app_info.yaml`，字段名和分类 ID
与 App Store Connect 的 relationship 保持一致：

```yaml
primaryCategory: GAMES
primarySubcategoryOne: GAMES_ACTION
secondaryCategory: ENTERTAINMENT
```

该文件支持主分类、次分类及各自两个可选子分类。`push` 时，文件中省略的字段会保持
远端原值不变。

对于 App Store 截图，`pull` 会把同步状态集中写入
`<bundle-id>/.manifest.yaml`，下载的图片则按顺序命名，例如 `001.png`。
`push` 会根据远端 ID 和校验值复用未变化的截图，替换内容已修改或处理失败的截图，
删除非空本地显示类型目录中已经不存在的远端截图，并按照本地文件名重新排序。
push 成功后，最终的远端 ID 和本地校验值会写回应用级 manifest。同步前可以使用独立命令
`fastforge appstore catalog push --app <bundle-id> --dry-run` 检查本地截图集。

---

## 资源说明

### `distribute_options.yaml`

完整的配置参考请查看[分发选项](./distribute-options.md)页面。
