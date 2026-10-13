# 本地工作流

[English](../en/workflows.md) | 简体中文

`fastforge workflow` 在本地发现、校验和运行 YAML 工作流，约定放在 `.fastforge/workflows/` 下。它适合把构建、打包、发布和普通 shell 步骤组织成可重复执行的任务。

## 目录结构

```text
your-project/
├── .fastforge/
│   └── workflows/
│       ├── android.yml
│       └── release.yml
└── project-files
```

## 最小工作流

```yaml
name: Android package

on:
  workflow_dispatch:
    inputs:
      flavor:
        description: Build flavor
        default: production

jobs:
  package:
    name: Package Android app
    steps:
      - name: Create APK
        uses: fastforge/package
        with:
          platform: android
          target: apk
          output: dist/
          build-args: '{"flavor":"${{ inputs.flavor }}"}'
```

## 发现工作流

```bash
fastforge workflow list
fastforge workflow list --verbose
fastforge workflow list --dir /path/to/project
```

发现过程会依次读取项目（`run` 命令为 `--workspace`）下三个目录中的 `*.yml` 和 `*.yaml` 文件：

1. `.fastforge/workflows/`
2. `.minact/workflows/`
3. `.github/workflows/`

因此 GitHub Actions 文件也会被计入：已有 CI 工作流的项目通常需要 `--file`；如果项目中唯一的工作流位于 `.github/workflows/`，直接执行 `fastforge workflow run` 会在本地运行该文件。解析失败的文件会被跳过并输出警告，不会出现在 `list` 结果中；可以对其运行 `validate` 查看错误。

## 校验工作流

```bash
fastforge workflow validate .fastforge/workflows/release.yml
```

校验会解析 YAML 并检查工作流结构：至少包含一个 job、每个 job 都有 step、每个 step 只能有 `uses` 或 `run` 之一。文件无效时以非零状态退出。校验不执行命令或 action，action 的 input（例如必填的 `platform`/`target`，或 `build-args` 是否为有效 JSON）要到运行时才会检查。

## 运行工作流

发现过程恰好找到一个工作流时：

```bash
fastforge workflow run
```

找到多个工作流时必须指定文件（路径相对于当前目录）：

```bash
fastforge workflow run --file .fastforge/workflows/release.yml
```

传入 `workflow_dispatch` input：

```bash
fastforge workflow run \
  --file .fastforge/workflows/release.yml \
  --input flavor=staging \
  --input channel=beta
```

模拟事件或改变工作目录：

```bash
fastforge workflow run \
  --file .fastforge/workflows/release.yml \
  --event push \
  --workspace /path/to/project
```

## `fastforge/package` action

必填 input：

| Input      | 说明     |
| ---------- | -------- |
| `platform` | 目标平台 |
| `target`   | 打包格式 |

可选 input：

| Input           | 说明                           |
| --------------- | ------------------------------ |
| `output`        | 输出目录，默认 `dist/`         |
| `artifact-name` | 产物名称模板                   |
| `skip-clean`    | 字符串 `true` 时跳过构建前清理 |
| `channel`       | 用于产物名称的渠道名           |
| `build-target`  | Flutter Builder 的入口文件     |
| `build-args`    | JSON object 字符串             |
| `hook-pre`      | 打包前 shell 命令              |
| `hook-post`     | 打包后 shell 命令              |
| `notarize`      | 字符串 `true` 时公证 macOS DMG、PKG 和 ZIP |

`skip-clean` 和 `channel` 只对 Flutter 项目生效，原生 Gradle、Xcode 和 .NET 项目会忽略它们。与 `fastforge package` 不同，该 action 不读取 `distribute_options.yaml`：输出目录来自 `output`，变量来自工作流、job 和 step 的 `env` 与进程环境变量的合并结果（冲突时以进程环境变量为准）。

`build-args` 的字段由实际构建器决定，分别见 [Gradle Builder](builders/gradle.md)、[Xcode Builder](builders/xcode.md)、[.NET Builder](builders/dotnet.md) 和 [Flutter Builder](builders/flutter.md)。

示例：

```yaml
- name: Package
  uses: fastforge/package
  with:
    platform: android
    target: aab
    output: artifacts/
    artifact-name: "my-app-{{build_name}}.{{ext}}"
    build-args: '{"flavor":"production","module":"app"}'
    hook-post: ./scripts/verify-artifact.sh
```

Action 输出：

- `artifact-count`
- `artifact-paths`（逗号分隔）

## `fastforge/publish` action

必填 input：

| Input    | 说明               |
| -------- | ------------------ |
| `path`   | 要发布的文件或目录 |
| `target` | 发布目标           |

可以把发布参数集中写成 JSON object，其中所有值都必须是字符串（数字和布尔值也要加引号，例如 `"draft":"true"`）：

```yaml
- name: Publish
  uses: fastforge/publish
  with:
    path: dist/app.zip
    target: github
    publish-args: '{"repo":"owner/repository","release-tag":"v1.0.0"}'
```

如果省略 `publish-args`，除 `path` 和 `target` 外的其他 `with` 字段都会作为发布参数：

```yaml
- name: Publish
  uses: fastforge/publish
  with:
    path: dist/app.zip
    target: github
    repo: owner/repository
    release-tag: v1.0.0
```

Action 输出为 `message`。

## 支持的语法

引擎遵循 GitHub Actions 语法。除两个 Fastforge action 外，还支持：

- `run` step（支持 `shell`、`working-directory`、`env`），以及 step 上的 `if`、`continue-on-error` 和 `timeout-minutes`
- job 的 `needs`、`if`、`outputs` 和 `strategy.matrix`；`${{ }}` 表达式可访问 `inputs`、`env`、`steps`、`needs`、`matrix`、`strategy`、`job`、`runner` 和 `github`（`secrets` 和 `vars` 始终为空字符串）
- `uses:` 依次解析为内置 action（`fastforge/package`、`fastforge/publish`、`actions/checkout`、`actions/cache`、`actions/upload-artifact`、`actions/download-artifact`）、带 `action.yml` 的本地 `./path` action、`docker://image`，或远程 `owner/repo@ref` action。远程 action 需要联网获取，并缓存在 `~/.minact/actions`。

job 在本机运行。`runs-on` 不会选择 runner：与本机不符的标签只会输出警告。带 `container:` 的 job 会在对应的 Docker 镜像中执行 step，`services:` 不会启动。引擎把临时文件放在工作区下的 `.minact/_work/` 中。

Fastforge 尚未集成的构建系统可以作为 `run` step 执行，再把产物路径交给 `fastforge/publish` step。

## 执行结果

工作流引擎会按依赖关系生成执行层，逐步输出 job、step、命令和 action 状态。任何失败会使最终结果失败并以非零状态退出，适合直接用于本地脚本或 CI。

## 注意事项

- `build-args` 和 `publish-args` 必须是有效 JSON，不是 YAML object。
- 工作流内置 package action 与 CLI 使用相同的当前打包覆盖范围。
- 发布凭证应通过运行进程的环境变量传入。`fastforge/publish` 只读取进程环境变量，不读取工作流中的 `env:`；`with` 的值会打印在运行日志中。
- 多个工作流并存时显式使用 `--file`，避免选择歧义。
