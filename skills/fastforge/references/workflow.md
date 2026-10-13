# Fastforge Local Workflows

`fastforge workflow` discovers, validates, and runs YAML workflows,
conventionally kept in `.fastforge/workflows/`. The syntax mirrors GitHub
Actions (name / on / jobs / steps / `${{ }}` expressions) and runs on the local
machine; the two Fastforge actions below take only the inputs documented here.

```text
your-project/
├── .fastforge/
│   └── workflows/
│       ├── android.yml
│       └── release.yml
└── project-files
```

## Minimal workflow

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

## CLI

```bash
fastforge workflow list [--verbose] [--dir <path>]
fastforge workflow validate .fastforge/workflows/release.yml
fastforge workflow run                             # OK only when discovery finds exactly one
fastforge workflow run --file .fastforge/workflows/release.yml \
  --input flavor=staging --input channel=beta      # workflow_dispatch inputs
fastforge workflow run --file <file> --event push --workspace /path/to/project
```

- Discovery (for `list`, and `run` without `--file`) reads `*.yml`/`*.yaml` from
  `.fastforge/workflows/`, `.minact/workflows/`, **and `.github/workflows/`**
  under `--dir` / `--workspace` (default: cwd). A project with GitHub Actions CI
  therefore usually needs `--file`; with no Fastforge workflow but one GitHub
  workflow, a bare `run` executes that GitHub workflow locally.
- Files that fail to parse are skipped during discovery (only a log warning), so
  a broken file silently disappears from `list` — run `validate` on it.
- `--file` is relative to the current directory, not `--workspace`.
- `--event` defaults to `workflow_dispatch`; `--input` values override declared
  `inputs.<name>.default`.
- `validate` checks YAML plus structure (≥1 job, every job has steps, each step
  has exactly one of `uses`/`run`) and exits 1 when invalid. It does not run
  anything or check action inputs — missing `platform`, or `build-args` that is
  not JSON, only fails at run time.

## `fastforge/package` action

Required: `platform`, `target`.

| Optional input | Description |
| --- | --- |
| `output` | Output directory; defaults to `dist/` |
| `artifact-name` | Artifact name template, e.g. `"my-app-{{build_name}}.{{ext}}"` |
| `skip-clean` | Skip cleaning when the string is `true` (Flutter projects only) |
| `build-target` | Flutter Builder entry point (e.g. `lib/main_prod.dart`) |
| `build-args` | **JSON object string** — fields depend on the active builder |
| `channel` | Channel name used in artifact naming (Flutter projects only; native Gradle/Xcode ignore it) |
| `hook-pre` / `hook-post` | Shell command before/after packaging |
| `notarize` | `"true"` notarizes and staples macOS DMG, PKG, and ZIP |

Outputs: `artifact-count`, `artifact-paths` (comma-separated).

Unlike `fastforge package`, the action does not read `distribute_options.yaml`:
output comes from `output`, variables from the process environment.

Which `build-args` fields exist depends on the builder the project routes to
(Gradle, Xcode, or Flutter) — the fastforge-package skill's references
document them per platform.

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

## `fastforge/publish` action

Required: `path`, `target`. Parameters go either in a `publish-args` JSON
object string whose values are **all strings** (`'{"draft":"true"}'`, not
`true`), or — when `publish-args` is omitted — every other `with` field except
`path`/`target` becomes a publishing parameter:

```yaml
- name: Publish
  uses: fastforge/publish
  with:
    path: dist/app.zip
    target: github
    repo: owner/repository
    release-tag: v1.0.0
```

Output: `message`.

## Other supported syntax

Beyond the Fastforge actions, the engine (minact) supports `run` steps with
`shell` / `working-directory` / `env`, step `if` / `continue-on-error` /
`timeout-minutes`, job `needs` / `if` / `outputs` / `strategy.matrix`, and
expressions over `inputs`, `env`, `steps`, `needs`, `matrix`, `github`. A `uses:`
that is not a built-in (`fastforge/*`, `actions/checkout`, `actions/cache`,
`actions/upload-artifact`, `actions/download-artifact`) resolves to a local
`./path` action, `docker://image`, or a remote `owner/repo@ref` fetched over the
network into `~/.minact/actions`.

## Execution model

The engine builds execution layers from job dependencies and reports job,
step, command, and action status as it runs. Any failure makes the overall run
fail with a nonzero exit status, so workflows compose cleanly with scripts and
CI.

## Rules that prevent the common failures

- `build-args` and `publish-args` must be valid **JSON**, not YAML objects.
  Quote the whole value: `build-args: '{"flavor":"dev"}'`.
- The package action has exactly the same platform/format coverage as the CLI
  — a format the CLI cannot package does not start working inside a workflow.
- Publishing credentials come from process environment variables; only
  non-sensitive parameters belong in `with`.
- Steps may also be plain shell commands (`run:` instead of `uses:`), which is
  how unsupported build systems are integrated today: run the custom build as a
  shell step, then pass its artifact path to a `fastforge/publish` step.
- A YAML map under `with` does not error at parse time — it is serialized back
  to YAML text and then fails as invalid JSON when the step runs.
