# CLI 参考

[English](../en/cli.md) | 简体中文

```text
fastforge <COMMAND>
```

全局参数：

| 参数            | 说明     |
| --------------- | -------- |
| `-h, --help`    | 显示帮助 |
| `-V, --version` | 显示版本 |

顶层命令：

| 命令            | 说明                          |
| --------------- | ----------------------------- |
| `analyze`       | 分析应用包，或整个目录        |
| `build`         | 使用 Flutter Builder 构建项目 |
| `package`       | 构建并打包项目                |
| `publish`       | 发布现有产物                  |
| `release`       | 兼容旧版发布流程              |
| `store`         | 管理聚合商店配置与 catalog    |
| `workflow`      | 运行本地工作流                |
| `appstore`      | 操作 App Store Connect        |
| `appgallery`    | 操作华为 AppGallery Connect   |
| `googleplay`    | 操作 Google Play Console      |
| `upgrade`       | 预留的升级命令                |
| `version-check` | 输出当前版本                  |

## `analyze`

```text
fastforge analyze [OPTIONS] <PATH>...
```

| 参数                    | 必填 | 说明                                                              |
| ----------------------- | :--: | ----------------------------------------------------------------- |
| `<PATH>...`             |  是  | 一个或多个 `.apk`、`.aab`、`.ipa`、`.dmg`、`.app` 路径，或待扫描的目录 |
| `-o, --output <OUTPUT>` |  否  | 把报告写入文件；省略时输出到 stdout                               |
| `--format <FORMAT>`     |  否  | `json` 或 `html`；默认取 `--output` 隐含的格式，否则为 `json`     |

```bash
fastforge analyze dist/app.apk
fastforge analyze dist/app.ipa --output app-info.json
fastforge analyze dist --output report.html
```

格式依赖和输出说明见[应用包分析](tools/analyze.md)。

## `build`

```text
fastforge build [OPTIONS]
```

| 参数                                  | 说明                       |
| ------------------------------------- | -------------------------- |
| `-p, --platform <PLATFORM>`           | 目标平台；执行时必填       |
| `-t, --target <TARGET>`               | 构建 target                |
| `--clean`                             | 构建前执行清理             |
| `--flutter-build-args <ARGS>`         | Flutter Builder 的额外参数 |
| `--build-target <PATH>`               | Flutter 入口文件           |
| `--build-flavor <FLAVOR>`             | 构建 flavor                |
| `--build-target-platform <PLATFORM>`  | 构建目标架构               |
| `--build-export-options-plist <PATH>` | iOS ExportOptions plist    |
| `--build-export-method <METHOD>`      | iOS export method          |
| `--build-dart-define <KEY=VALUE>`     | 编译变量；可重复           |
| `--build-obfuscate`                   | 开启 obfuscate             |
| `--build-split-debug-info <PATH>`     | 调试符号输出目录           |
| `--build-tree-shake-icons`            | 开启 icon tree shaking     |
| `--build-profile`                     | 使用 profile 模式          |

当前 `build` 命令的适用范围和构建器状态见[构建](building.md)。

## `package`

```text
fastforge package [OPTIONS]
```

| 参数                        | 说明                        |
| --------------------------- | --------------------------- |
| `-p, --platform <PLATFORM>` | 目标平台；执行时必填        |
| `-t, --target <TARGET>`     | 单个打包 target；执行时必填 |
| `--skip-clean`              | 跳过构建前清理              |
| `--build-target <PATH>`     | Flutter Builder 的入口文件  |
| `--hook-pre <COMMAND>`      | 打包前 shell 命令           |
| `--hook-post <COMMAND>`     | 打包后 shell 命令           |

当前支持范围见[打包](packaging.md)。

各平台和格式说明见[打包器总览](packagers/README.md)。

## `publish`

```text
fastforge publish [OPTIONS]
```

| 参数                        | 说明                        |
| --------------------------- | --------------------------- |
| `--path <PATH>`             | 文件或目录路径；执行时必填  |
| `-t, --target <TARGET>`     | 单个发布 target；执行时必填 |
| `--publish-arg <KEY=VALUE>` | 发布器参数；可重复          |

各 target 的凭证和参数见[发布器总览](publishers/README.md)。

## `release`

该命令保留用于兼容旧版发布流程。新自动化流程使用 `fastforge workflow`。

## `store`

```text
fastforge store <COMMAND>
```

| 子命令         | 说明                                             |
| -------------- | ------------------------------------------------ |
| `list`         | 列出 `.fastforge/config.yaml` 中配置的商店与应用 |
| `catalog pull` | 拉取全部已配置应用的 catalog                     |
| `catalog push` | 推送全部已配置应用的 catalog                     |

## `workflow`

### Run

```text
fastforge workflow run [OPTIONS]
```

| 参数                          | 说明                               |
| ----------------------------- | ---------------------------------- |
| `-f, --file <FILE>`           | 指定工作流文件                     |
| `-e, --event <EVENT>`         | 模拟事件，默认 `workflow_dispatch` |
| `-w, --workspace <WORKSPACE>` | 工作目录，默认当前目录             |
| `-i, --input <KEY=VALUE>`     | input；可重复                      |

### List

```text
fastforge workflow list [OPTIONS]
```

| 参数              | 说明         |
| ----------------- | ------------ |
| `-d, --dir <DIR>` | 搜索目录     |
| `-v, --verbose`   | 显示详细信息 |

### Validate

```text
fastforge workflow validate <FILE>
```

## `appstore`

```text
fastforge appstore [GLOBAL OPTIONS] <COMMAND>
```

| 命令组       | 子命令                                                                           |
| ------------ | -------------------------------------------------------------------------------- |
| `app`        | `list`、`view`                                                                   |
| `build`      | `list`、`view`、`upload`、`wait`                                                 |
| `version`    | `list`、`view`、`submit`                                                         |
| `submission` | `list`、`view`、`create`、`items`、`add-item`、`remove-item`、`submit`、`cancel` |
| `catalog`    | `pull`、`push`                                                                   |
| `api`        | `get`、`post`、`patch`、`delete`                                                 |

全局参数：

- `--json <FIELDS>`
- `--limit <LIMIT>`
- `--paginate`
- `--verbose`
- `--debug`
- `--no-color`

## `appgallery`

```text
fastforge appgallery [GLOBAL OPTIONS] <COMMAND>
```

| 命令组    | 子命令                                  |
| --------- | --------------------------------------- |
| `app`     | `resolve`、`view`                       |
| `package` | `list`、`status`                        |
| `release` | 提交应用审核                            |
| `api`     | `get`、`post`、`put`、`patch`、`delete` |

认证和示例见 [AppGallery Connect](stores/appgallery.md)。

## `googleplay`

```text
fastforge googleplay [GLOBAL OPTIONS] <COMMAND>
```

| 命令组    | 子命令                                  |
| --------- | --------------------------------------- |
| `app`     | `view`、`check`                         |
| `edit`    | `create`、`commit`、`delete`            |
| `bundle`  | `upload`                                |
| `track`   | `list`、`view`、`update`                |
| `catalog` | `pull`、`push`                          |
| `api`     | `get`、`post`、`put`、`patch`、`delete` |

全局参数：

- `--json <FIELDS>`
- `--limit <LIMIT>`
- `--verbose`
- `--debug`
- `--no-color`

商店子命令参数较多，使用逐层帮助查看当前定义：

```bash
fastforge appstore build upload --help
fastforge appstore submission create --help
fastforge appgallery app resolve --help
fastforge googleplay bundle upload --help
fastforge googleplay track update --help
```

## `version-check`

```text
fastforge version-check [--current-only]
```

当前实现只打印编译时版本，无论是否传入 `--current-only` 都不会联网检查新版本。

## `upgrade`

```text
fastforge upgrade
```

当前命令为空操作，不会下载或替换二进制。升级请重新运行安装脚本或安装指定版本。
