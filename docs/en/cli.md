# CLI Reference

English | [简体中文](../zh-Hans/cli.md)

```text
fastforge <COMMAND>
```

Global options:

| Option               | Description                                                             |
| -------------------- | ----------------------------------------------------------------------- |
| `-h, --help`         | Show help                                                               |
| `-V, --version`      | Show the version                                                        |
| `--no-version-check` | Skip the update check that runs before every command (on by default)    |
| `--version-check`    | Re-enable the update check (overrides an earlier `--no-version-check`) |

Before running a command, fastforge checks GitHub releases for a newer version and prints an upgrade hint (or a "latest version" note) to stderr. The check times out after 5 seconds and never fails the command. Set `GITHUB_TOKEN` to avoid GitHub API rate limits.

Top-level commands:

| Command         | Description                                        |
| --------------- | -------------------------------------------------- |
| `analyze`       | Analyze app packages, or a directory of them       |
| `build`         | Build a project with Flutter Builder               |
| `host`          | Manage remote hosts (see [Remote Hosts](remote-hosts.md)) |
| `package`       | Build and package a project                        |
| `run`           | Run the app on a device, like `flutter run`        |
| `publish`       | Publish an existing artifact                       |
| `release`       | Preserve compatibility with legacy releases        |
| `store`         | Manage aggregated store configuration and catalogs |
| `studio`        | Open Studio to manage local projects               |
| `workflow`      | Run local workflows                                |
| `appstore`      | Operate App Store Connect                          |
| `appgallery`    | Operate Huawei AppGallery Connect                  |
| `googleplay`    | Operate Google Play Console                        |
| `upgrade`       | Upgrade fastforge to the latest release            |
| `version-check` | Check for a newer version                          |

## `studio`

```bash
fastforge studio
fastforge studio --no-open --port 7391
fastforge studio serve --web-root /path/to/dist/client
fastforge studio doctor --dir /path/to/project
```

Without a subcommand, Studio starts its local server and opens the browser. `serve` explicitly starts the same server. Both accept `--port` (default `7391`), `--no-open`, and `--web-root`. `doctor` checks the project’s store credentials and accepts `--dir` (defaults to the current directory).

In a source checkout, run `pnpm studio:build` first to build the web client, or run `pnpm studio:dev` separately during development. See the [Studio development guide](../../apps/studio-cli/README.md).

## `analyze`

```text
fastforge analyze [OPTIONS] <PATH>...
```

| Option                  | Required | Description                                                                    |
| ----------------------- | :------: | ------------------------------------------------------------------------------ |
| `<PATH>...`             |   Yes    | One or more `.apk`, `.aab`, `.ipa`, `.dmg` or `.app` paths, or directories to scan |
| `-o, --output <OUTPUT>` |    No    | Write the report to a file; otherwise write to stdout                          |
| `--format <FORMAT>`     |    No    | `json` or `html`. Defaults to the format implied by `--output`, else `json`    |

```bash
fastforge analyze dist/app.apk
fastforge analyze dist/app.ipa --output app-info.json
fastforge analyze dist --output report.html
```

See [App Package Analysis](tools/analyze.md) for format dependencies and output details.

## `build`

```text
fastforge build [OPTIONS]
```

| Option                                | Description                          |
| ------------------------------------- | ------------------------------------ |
| `-p, --platform <PLATFORM>`           | Target platform; inferred from the target and project when omitted |
| `-t, --target <TARGET>`               | Build target                         |
| `--clean`                             | Clean before building                |
| `--flutter-build-args <ARGS>`         | Additional Flutter Builder arguments |
| `--build-target <PATH>`               | Flutter entry point                  |
| `--build-flavor <FLAVOR>`             | Build flavor                         |
| `--build-target-platform <PLATFORM>`  | Target architecture                  |
| `--build-export-options-plist <PATH>` | iOS ExportOptions plist              |
| `--build-export-method <METHOD>`      | iOS export method                    |
| `--build-dart-define <KEY=VALUE>`     | Compile-time variable; repeatable    |
| `--build-obfuscate`                   | Enable obfuscation                   |
| `--build-split-debug-info <PATH>`     | Debug-symbol output directory        |
| `--build-tree-shake-icons`            | Enable icon tree shaking             |
| `--build-profile`                     | Use profile mode                     |

See [Building](building.md) for the current scope of the `build` command and builder status.

## `package`

```text
fastforge package [OPTIONS]
```

| Option                                | Description                                                                 |
| ------------------------------------- | --------------------------------------------------------------------------- |
| `-p, --platform <PLATFORM>`           | Target platform; inferred from the targets and project when omitted         |
| `-t, --targets <TARGET,...>`          | Comma-separated package targets (alias `--target`); required                |
| `--channel <CHANNEL>`                 | Channel name used in the artifact name                                      |
| `--artifact-name <TEMPLATE>`          | Mustache artifact-name template                                             |
| `--output <DIR>`                      | Directory the artifacts are written to (default: `output` of `distribute_options.yaml`, else `dist/`) |
| `--skip-clean`                        | Skip `flutter clean` before the build                                       |
| `--flutter-build-args <ARG,...>`      | Arguments passed to `flutter build` (`verbose,obfuscate`, `key=value`)      |
| `--build-target <PATH>`               | `--target` passed to `flutter build`                                        |
| `--build-flavor <FLAVOR>`             | `--flavor` passed to `flutter build`                                        |
| `--build-target-platform <PLATFORM>`  | `--target-platform` passed to `flutter build`                               |
| `--build-export-options-plist <PATH>` | `--export-options-plist` passed to `flutter build`                          |
| `--build-dart-define <KEY=VALUE>`     | `--dart-define` passed to `flutter build`; repeatable                       |
| `--hook-pre <COMMAND>`                | Shell command to run before packaging                                       |
| `--hook-post <COMMAND>`               | Shell command to run after packaging                                        |
| `--host <NAME\|auto>`                 | Package on a remote host and fetch the artifacts into the output directory |

> [!WARNING]
> `distribute_options.yaml` is deprecated. It is still read for compatibility with the Dart CLI, but it gets no new features. Use `--output` and environment variables with `package`, and [local workflows](workflows.md) instead of `release`.

Like the Dart CLI, `package` reads `distribute_options.yaml` when present: artifacts go to its `output` directory (default `dist/`), and its `variables` are layered over the environment for the build, the packagers (for example `INNO_SETUP_PATH`) and the hooks. `flutter clean` runs at most once; non-Android platforms build once and reuse the output for every target. A target whose builder cannot run on the current OS is skipped with a warning.

With `--host`, the project is synced to the host, the same `package` command runs there, and the artifacts are fetched into the local output directory. See [Remote Hosts](remote-hosts.md).

See [Packaging](packaging.md) for current support.

See the [packager overview](packagers/README.md) for platform and format details.

## `host`

```text
fastforge host add <NAME> [DESTINATION] [OPTIONS]
fastforge host list
fastforge host remove <NAME>
fastforge host doctor <NAME>
fastforge host exec <NAME> [--no-sync] -- <COMMAND>...
```

| Subcommand | Description |
| ---------- | ----------- |
| `add`      | Add a host to `~/.fastforge/hosts.yaml`. `DESTINATION` is `[user@]host[:port]` or an `~/.ssh/config` alias. Options: `--identity-file`, `--workdir`, `--fastforge`, `--platforms`, `--forward-env`, `--env KEY=VALUE` (repeatable), `--transport ssh\|local`, `--os unix\|windows`, `--force` |
| `list`     | List the configured hosts |
| `remove`   | Remove a host |
| `doctor`   | Check the connection, the remote fastforge and its tools; saves the detected OS and platforms when not configured |
| `exec`     | Sync the current project and run a shell command in the remote workspace |

See [Remote Hosts](remote-hosts.md).

## `run`

```text
fastforge run [OPTIONS] [-- <FLUTTER_ARGS>...]
```

| Option                           | Description                                                              |
| -------------------------------- | ------------------------------------------------------------------------ |
| `-p, --platform <PLATFORM>`      | Platform to run on; picks the first matching device when `-d` is omitted (Chrome for `web`) |
| `-d, --device-id <ID>`           | Device id or name, as listed by `flutter devices`                        |
| `--release` / `--profile`        | Build mode (default debug)                                               |
| `--flavor <FLAVOR>`              | Build flavor                                                             |
| `-t, --target <PATH>`            | Entry-point file (default `lib/main.dart`)                               |
| `--dart-define <KEY=VALUE>`      | Compile-time variable; repeatable                                        |
| `--dart-define-from-file <PATH>` | File of compile-time variables                                           |
| `--host <NAME\|auto>`            | Run on a remote host (see [Remote Hosts](remote-hosts.md#running-the-app-on-a-host)) |

Runs `flutter run` with these options, plus everything after `--`. Without `-p` and `-d`, Flutter picks the device. The environment is passed through unchanged; `distribute_options.yaml` is not read. Only Flutter projects are supported for now.

## `publish`

```text
fastforge publish [OPTIONS]
```

| Option                         | Description                                                |
| ------------------------------ | ---------------------------------------------------------- |
| `--path <PATH>`                | File or directory path; required                           |
| `-t, --targets <TARGET,...>`   | Comma-separated publishing targets (alias `--target`)      |
| `--app-version <VERSION>`      | App version passed to publishers                           |
| `--publish-arg <KEY=VALUE>`    | Publisher argument; repeatable                             |

The provider options of the Dart CLI are also accepted and forwarded to the matching publisher with their prefix stripped (`--github-repo` becomes `repo` for `github`): `--appgallery-app-id`, `--firebase-app`, `--firebase-release-notes[-file]`, `--firebase-testers[-file]`, `--firebase-groups[-file]`, `--firebase-hosting-project-id`, `--github-repo`, `--github-repo-owner`, `--github-repo-name`, `--github-release-title`, `--github-release-draft`, `--github-release-prerelease`, `--minio-endpoint`, `--minio-access-key`, `--minio-secret-key`, `--minio-region`, `--minio-bucket`, `--minio-savekey-prefix`, `--pgyer-*`, `--playstore-package-name`, `--playstore-track`, `--qiniu-bucket`, `--qiniu-bucket-domain`, `--qiniu-savekey-prefix`, `--vercel-org-id` and `--vercel-project-id`. `--firebase-app` is required for the `firebase` target. Publishers read credentials from the environment plus the `variables` in `distribute_options.yaml`.

See the [publisher overview](publishers/README.md) for credentials and arguments for each target.

## `release`

```text
fastforge release [--name <NAME>] [--jobs <JOB,...>] [--skip-jobs <JOB,...>] [--skip-clean] [--dry-run]
```

> [!WARNING]
> Deprecated together with `distribute_options.yaml`: kept for compatibility, no new features. Use [local workflows](workflows.md).

Runs the releases defined in `distribute_options.yaml`: every release when `--name` is omitted, otherwise the named one. `--jobs` selects jobs and takes precedence over `--skip-jobs`. Each job packages its target and, when `publish`/`publish_to` is set, publishes the first artifact. Variables are merged as environment < global `variables` < release `variables` < job `variables`. `flutter clean` runs at most once per release. The run ends with `RELEASE SUCCESSFUL in Ns` or `RELEASE FAILED in Ns`. For new automation, prefer `fastforge workflow`.

## `store`

```text
fastforge store <COMMAND>
```

| Subcommand     | Description                                                 |
| -------------- | ----------------------------------------------------------- |
| `list`         | List stores and apps configured in `.fastforge/config.yaml` |
| `catalog pull` | Pull catalogs for all configured apps                       |
| `catalog push` | Push catalogs for all configured apps                       |

## `workflow`

### Run

```text
fastforge workflow run [OPTIONS]
```

| Option                        | Description                                          |
| ----------------------------- | ---------------------------------------------------- |
| `-f, --file <FILE>`           | Select a workflow file                               |
| `-e, --event <EVENT>`         | Simulate an event; defaults to `workflow_dispatch`   |
| `-w, --workspace <WORKSPACE>` | Working directory; defaults to the current directory |
| `-i, --input <KEY=VALUE>`     | Input; repeatable                                    |

Without `--file`, workflows are discovered in `.fastforge/workflows/`, `.minact/workflows/` and `.github/workflows/` under the workspace; the command runs only when exactly one is found. See [Local Workflows](workflows.md#discover-workflows).

### List

```text
fastforge workflow list [OPTIONS]
```

| Option            | Description      |
| ----------------- | ---------------- |
| `-d, --dir <DIR>` | Search directory |
| `-v, --verbose`   | Show details     |

### Validate

```text
fastforge workflow validate <FILE>
```

Checks the YAML and its structure (at least one job, every job has steps, each step has exactly one of `uses` or `run`) and exits nonzero when invalid. Action inputs are not checked until the workflow runs.

## `appstore`

```text
fastforge appstore [GLOBAL OPTIONS] <COMMAND>
```

| Command group | Subcommands                                                                      |
| ------------- | -------------------------------------------------------------------------------- |
| `app`         | `list`, `view`                                                                   |
| `build`       | `list`, `view`, `upload`, `wait`                                                 |
| `version`     | `list`, `view`, `submit`                                                         |
| `submission`  | `list`, `view`, `create`, `items`, `add-item`, `remove-item`, `submit`, `cancel` |
| `catalog`     | `pull`, `push`                                                                   |
| `api`         | `get`, `post`, `patch`, `delete`                                                 |

Global options:

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

| Command group | Subcommands                             |
| ------------- | --------------------------------------- |
| `app`         | `resolve`, `view`                       |
| `package`     | `list`, `status`                        |
| `release`     | submit an app for review                |
| `api`         | `get`, `post`, `put`, `patch`, `delete` |

See [AppGallery Connect](stores/appgallery.md) for authentication and examples.

## `googleplay`

```text
fastforge googleplay [GLOBAL OPTIONS] <COMMAND>
```

| Command group | Subcommands                             |
| ------------- | --------------------------------------- |
| `app`         | `view`, `check`                         |
| `edit`        | `create`, `commit`, `delete`            |
| `bundle`      | `upload`                                |
| `track`       | `list`, `view`, `update`                |
| `catalog`     | `pull`, `push`                          |
| `api`         | `get`, `post`, `put`, `patch`, `delete` |

Global options:

- `--json <FIELDS>`
- `--limit <LIMIT>`
- `--verbose`
- `--debug`
- `--no-color`

Store subcommands have many options. Use help at each command level to inspect the current definitions:

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

Queries GitHub releases for the newest published version that ships a prebuilt binary for the current platform and reports whether an upgrade is available. With `--current-only`, prints only the local version without touching the network.

## `upgrade`

```text
fastforge upgrade [--force]
```

Downloads the latest release archive for the current platform (the same `fastforge-<version>-<target>` archive the installation scripts use) and replaces the running binary in place. It does nothing when the current version is already the latest; `--force` reinstalls anyway. If the binary lives in a directory you cannot write to (for example `/usr/local/bin`), re-run with elevated permissions or use the installation script.
