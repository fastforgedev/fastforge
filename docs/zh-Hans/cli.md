# CLI 参考

[English](../en/cli.md) | 简体中文

```text
fastforge <COMMAND>
```

全局参数：

| 参数                 | 说明                                                     |
| -------------------- | -------------------------------------------------------- |
| `-h, --help`         | 显示帮助                                                 |
| `-V, --version`      | 显示版本                                                 |
| `--no-version-check` | 跳过每次运行命令前的更新检查（默认开启）                 |
| `--version-check`    | 重新开启更新检查（覆盖前面的 `--no-version-check`）      |

运行命令前，fastforge 会查询 GitHub Releases 是否有新版本，并在 stderr 打印升级提示（或"已是最新版本"）。检查 5 秒超时，失败不会影响命令执行。设置 `GITHUB_TOKEN` 可避免 GitHub API 限流。

顶层命令：

| 命令            | 说明                          |
| --------------- | ----------------------------- |
| `analyze`       | 分析应用包，或整个目录        |
| `build`         | 使用 Flutter Builder 构建项目 |
| `package`       | 构建并打包项目                |
| `publish`       | 发布现有产物                  |
| `release`       | 兼容旧版发布流程              |
| `store`         | 管理聚合商店配置与 catalog    |
| `studio`        | 打开 Studio 管理本地项目      |
| `workflow`      | 运行本地工作流                |
| `appstore`      | 操作 App Store Connect        |
| `appgallery`    | 操作华为 AppGallery Connect   |
| `googleplay`    | 操作 Google Play Console      |
| `upgrade`       | 升级到最新版本                |
| `version-check` | 检查是否有新版本              |

## `studio`

```bash
fastforge studio
fastforge studio --no-open --port 7391
fastforge studio serve --web-root /path/to/dist/client
fastforge studio doctor --dir /path/to/project
```

不带子命令时，Studio 会启动本地服务并打开浏览器；`serve` 显式启动同一服务。两种方式均支持 `--port`（默认 `7391`）、`--no-open` 和 `--web-root`。`doctor` 检查项目的商店凭据，支持 `--dir`（默认为当前目录）。

从源码运行时，先执行 `pnpm studio:build` 构建 Web 界面，或在开发时另行运行 `pnpm studio:dev`。详见 [Studio 开发文档](../../apps/studio-cli/README.md)。

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
| `-p, --platform <PLATFORM>`           | 目标平台；省略时根据 target 和项目结构推断 |
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

| 参数                                  | 说明                                                             |
| ------------------------------------- | ---------------------------------------------------------------- |
| `-p, --platform <PLATFORM>`           | 目标平台；省略时根据 target 和项目结构推断                       |
| `-t, --targets <TARGET,...>`          | 逗号分隔的打包 target（别名 `--target`）；必填                   |
| `--channel <CHANNEL>`                 | 产物名中使用的渠道名                                             |
| `--artifact-name <TEMPLATE>`          | mustache 产物名模板                                              |
| `--output <DIR>`                      | 产物输出目录（默认取 `distribute_options.yaml` 的 `output`，否则 `dist/`） |
| `--skip-clean`                        | 构建前跳过 `flutter clean`                                       |
| `--flutter-build-args <ARG,...>`      | 传给 `flutter build` 的参数（`verbose,obfuscate`、`key=value`）  |
| `--build-target <PATH>`               | 传给 `flutter build` 的 `--target`                               |
| `--build-flavor <FLAVOR>`             | 传给 `flutter build` 的 `--flavor`                               |
| `--build-target-platform <PLATFORM>`  | 传给 `flutter build` 的 `--target-platform`                      |
| `--build-export-options-plist <PATH>` | 传给 `flutter build` 的 `--export-options-plist`                 |
| `--build-dart-define <KEY=VALUE>`     | 传给 `flutter build` 的 `--dart-define`；可重复                  |
| `--hook-pre <COMMAND>`                | 打包前 shell 命令                                                |
| `--hook-post <COMMAND>`               | 打包后 shell 命令                                                |

与 Dart 版一致，存在 `distribute_options.yaml` 时 `package` 会读取它：产物输出到其 `output` 目录（默认 `dist/`），其 `variables` 叠加在环境变量之上，传给构建、打包器（例如 `INNO_SETUP_PATH`）和 hook。`flutter clean` 最多执行一次；非 Android 平台只构建一次并复用给所有 target。构建器无法在当前系统运行的 target 会打印警告并跳过。

当前支持范围见[打包](packaging.md)。

各平台和格式说明见[打包器总览](packagers/README.md)。

## `publish`

```text
fastforge publish [OPTIONS]
```

| 参数                         | 说明                                        |
| ---------------------------- | ------------------------------------------- |
| `--path <PATH>`              | 文件或目录路径；必填                        |
| `-t, --targets <TARGET,...>` | 逗号分隔的发布 target（别名 `--target`）    |
| `--app-version <VERSION>`    | 传给发布器的应用版本                        |
| `--publish-arg <KEY=VALUE>`  | 发布器参数；可重复                          |

同时支持 Dart 版的各 provider 参数，会去掉前缀后传给对应发布器（例如 `github` 的 `--github-repo` 即 `repo`）：`--appgallery-app-id`、`--firebase-app`、`--firebase-release-notes[-file]`、`--firebase-testers[-file]`、`--firebase-groups[-file]`、`--firebase-hosting-project-id`、`--github-repo`、`--github-repo-owner`、`--github-repo-name`、`--github-release-title`、`--github-release-draft`、`--github-release-prerelease`、`--minio-endpoint`、`--minio-access-key`、`--minio-secret-key`、`--minio-region`、`--minio-bucket`、`--minio-savekey-prefix`、`--pgyer-*`、`--playstore-package-name`、`--playstore-track`、`--qiniu-bucket`、`--qiniu-bucket-domain`、`--qiniu-savekey-prefix`、`--vercel-org-id`、`--vercel-project-id`。发布到 `firebase` 时必须提供 `--firebase-app`。发布器从环境变量以及 `distribute_options.yaml` 的 `variables` 中读取凭证。

各 target 的凭证和参数见[发布器总览](publishers/README.md)。

## `release`

```text
fastforge release [--name <NAME>] [--jobs <JOB,...>] [--skip-jobs <JOB,...>] [--skip-clean] [--dry-run]
```

执行 `distribute_options.yaml` 中定义的 release：省略 `--name` 时执行全部 release，否则只执行指定的那个。`--jobs` 选择要执行的 job，优先于 `--skip-jobs`。每个 job 打包其 target，配置了 `publish`/`publish_to` 时发布第一个产物。变量合并顺序为：环境变量 < 全局 `variables` < release `variables` < job `variables`。每个 release 最多执行一次 `flutter clean`。结束时输出 `RELEASE SUCCESSFUL in Ns` 或 `RELEASE FAILED in Ns`。新的自动化流程建议使用 `fastforge workflow`。

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

未指定 `--file` 时，会在工作目录的 `.fastforge/workflows/`、`.minact/workflows/` 和 `.github/workflows/` 中发现工作流，且只有恰好找到一个时才会运行。详见[本地工作流](workflows.md#发现工作流)。

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

检查 YAML 语法和结构（至少一个 job、每个 job 都有 step、每个 step 只能有 `uses` 或 `run` 之一），无效时以非零状态退出。action 的 input 要到运行时才会检查。

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

查询 GitHub Releases 中已发布、且包含当前平台预编译二进制的最新版本，并提示是否可以升级。传入 `--current-only` 时只打印本地版本，不联网。

## `upgrade`

```text
fastforge upgrade [--force]
```

下载当前平台的最新发布包（与安装脚本使用的 `fastforge-<version>-<target>` 压缩包相同），并原地替换正在运行的二进制。当前已是最新版本时不做任何操作；`--force` 强制重新安装。若二进制所在目录没有写权限（例如 `/usr/local/bin`），请以提升的权限重新运行，或改用安装脚本。
