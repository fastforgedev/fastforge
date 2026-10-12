# Local Workflows

English | [简体中文](../zh-Hans/workflows.md)

`fastforge workflow` discovers, validates, and runs YAML workflows, conventionally kept under `.fastforge/workflows/`. Use it to organize build, package, publish, and ordinary shell steps into repeatable tasks.

## Directory Structure

```text
your-project/
├── .fastforge/
│   └── workflows/
│       ├── android.yml
│       └── release.yml
└── project-files
```

## Minimal Workflow

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

## Discover Workflows

```bash
fastforge workflow list
fastforge workflow list --verbose
fastforge workflow list --dir /path/to/project
```

Discovery reads `*.yml` and `*.yaml` files from three directories under the project (or `--workspace` for `run`), in this order:

1. `.fastforge/workflows/`
2. `.minact/workflows/`
3. `.github/workflows/`

GitHub Actions files therefore count too: a project with CI workflows usually needs `--file`, and a bare `fastforge workflow run` in a project whose only workflow is under `.github/workflows/` runs that file locally. Files that fail to parse are skipped with a warning and do not appear in `list`; run `validate` on them to see the error.

## Validate a Workflow

```bash
fastforge workflow validate .fastforge/workflows/release.yml
```

Validation parses the YAML and checks the workflow structure: at least one job, every job has steps, and each step has exactly one of `uses` or `run`. It exits with a nonzero status when the file is invalid. It does not run commands or actions, and action inputs (such as the required `platform`/`target`, or whether `build-args` is valid JSON) are only checked at run time.

## Run a Workflow

When discovery finds exactly one workflow:

```bash
fastforge workflow run
```

When several workflows are found, select a file explicitly (the path is relative to the current directory):

```bash
fastforge workflow run --file .fastforge/workflows/release.yml
```

Pass `workflow_dispatch` inputs:

```bash
fastforge workflow run \
  --file .fastforge/workflows/release.yml \
  --input flavor=staging \
  --input channel=beta
```

Simulate an event or change the working directory:

```bash
fastforge workflow run \
  --file .fastforge/workflows/release.yml \
  --event push \
  --workspace /path/to/project
```

## `fastforge/package` Action

Required inputs:

| Input      | Description     |
| ---------- | --------------- |
| `platform` | Target platform |
| `target`   | Package format  |

Optional inputs:

| Input           | Description                             |
| --------------- | --------------------------------------- |
| `output`        | Output directory; defaults to `dist/`   |
| `artifact-name` | Artifact name template                  |
| `skip-clean`    | Skip cleaning when the string is `true` |
| `channel`       | Channel name used in the artifact name  |
| `build-target`  | Flutter Builder entry point             |
| `build-args`    | JSON object string                      |
| `hook-pre`      | Shell command to run before packaging   |
| `hook-post`     | Shell command to run after packaging    |

`skip-clean` and `channel` only affect Flutter projects; native Gradle, Xcode, and .NET projects ignore them. Unlike `fastforge package`, the action does not read `distribute_options.yaml`: the output directory comes from `output`, and variables come from the workflow, job and step `env` merged with the process environment (the process environment wins on conflicts).

The active builder determines the fields accepted by `build-args`. See [Gradle Builder](builders/gradle.md), [Xcode Builder](builders/xcode.md), [.NET Builder](builders/dotnet.md), and [Flutter Builder](builders/flutter.md).

Example:

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

Action outputs:

- `artifact-count`
- `artifact-paths` (comma-separated)

## `fastforge/publish` Action

Required inputs:

| Input    | Description                  |
| -------- | ---------------------------- |
| `path`   | File or directory to publish |
| `target` | Publishing target            |

Publishing parameters can be grouped in a JSON object whose values are all strings (numbers and booleans must be quoted, e.g. `"draft":"true"`):

```yaml
- name: Publish
  uses: fastforge/publish
  with:
    path: dist/app.zip
    target: github
    publish-args: '{"repo":"owner/repository","release-tag":"v1.0.0"}'
```

If `publish-args` is omitted, every other `with` field except `path` and `target` becomes a publishing parameter:

```yaml
- name: Publish
  uses: fastforge/publish
  with:
    path: dist/app.zip
    target: github
    repo: owner/repository
    release-tag: v1.0.0
```

The action outputs `message`.

## Supported Syntax

The engine follows GitHub Actions syntax. Besides the two Fastforge actions it supports:

- `run` steps (with `shell`, `working-directory`, `env`), plus `if`, `continue-on-error`, and `timeout-minutes` on steps
- job `needs`, `if`, `outputs`, and `strategy.matrix`; `${{ }}` expressions over `inputs`, `env`, `steps`, `needs`, `matrix`, `strategy`, `job`, `runner`, and `github` (`secrets` and `vars` always evaluate to empty strings)
- `uses:` resolves, in order, to a built-in action (`fastforge/package`, `fastforge/publish`, `actions/checkout`, `actions/cache`, `actions/upload-artifact`, `actions/download-artifact`), a local `./path` action with an `action.yml`, a `docker://image`, or a remote `owner/repo@ref` action. Remote actions are fetched over the network and cached in `~/.minact/actions`.

Jobs run on this machine. `runs-on` does not select a runner: a label that doesn't match this machine only prints a warning. A job with `container:` runs its steps in that Docker image, and `services:` are not started. The engine keeps its scratch files in `.minact/_work/` under the workspace.

A build system Fastforge does not integrate can run as a `run` step, with its artifact path passed to a `fastforge/publish` step.

## Execution Result

The workflow engine creates execution layers from dependencies and reports job, step, command, and action status as it proceeds. Any failure makes the final result fail with a nonzero exit status, so workflows can be used directly from local scripts or CI.

## Notes

- `build-args` and `publish-args` must be valid JSON, not YAML objects.
- The built-in package action has the same current packaging coverage as the CLI.
- Pass publishing credentials through environment variables on the running process. `fastforge/publish` reads only the process environment, not `env:` from the workflow, and `with` values are printed in the run log.
- When multiple workflows exist, use `--file` explicitly to avoid ambiguity.
