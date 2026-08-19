# CLI Reference

English | [简体中文](../zh-Hans/cli.md)

```text
fastforge <COMMAND>
```

Global options:

| Option          | Description      |
| --------------- | ---------------- |
| `-h, --help`    | Show help        |
| `-V, --version` | Show the version |

Top-level commands:

| Command         | Description                                        |
| --------------- | -------------------------------------------------- |
| `analyze`       | Analyze app packages, or a directory of them       |
| `build`         | Build a project with Flutter Builder               |
| `package`       | Build and package a project                        |
| `publish`       | Publish an existing artifact                       |
| `release`       | Preserve compatibility with legacy releases        |
| `store`         | Manage aggregated store configuration and catalogs |
| `workflow`      | Run local workflows                                |
| `appstore`      | Operate App Store Connect                          |
| `appgallery`    | Operate Huawei AppGallery Connect                  |
| `googleplay`    | Operate Google Play Console                        |
| `upgrade`       | Reserved upgrade command                           |
| `version-check` | Print the current version                          |

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
| `-p, --platform <PLATFORM>`           | Target platform; required at runtime |
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

| Option                      | Description                             |
| --------------------------- | --------------------------------------- |
| `-p, --platform <PLATFORM>` | Target platform; required at runtime    |
| `-t, --target <TARGET>`     | One package target; required at runtime |
| `--skip-clean`              | Skip cleaning before the build          |
| `--build-target <PATH>`     | Flutter Builder entry point             |
| `--hook-pre <COMMAND>`      | Shell command to run before packaging   |
| `--hook-post <COMMAND>`     | Shell command to run after packaging    |

See [Packaging](packaging.md) for current support.

See the [packager overview](packagers/README.md) for platform and format details.

## `publish`

```text
fastforge publish [OPTIONS]
```

| Option                      | Description                                 |
| --------------------------- | ------------------------------------------- |
| `--path <PATH>`             | File or directory path; required at runtime |
| `-t, --target <TARGET>`     | One publishing target; required at runtime  |
| `--publish-arg <KEY=VALUE>` | Publisher argument; repeatable              |

See the [publisher overview](publishers/README.md) for credentials and arguments for each target.

## `release`

This command is retained for compatibility with the legacy release process. Use `fastforge workflow` for new automation.

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

The current implementation prints only the version embedded at compile time. It does not check the network for a newer version, with or without `--current-only`.

## `upgrade`

```text
fastforge upgrade
```

The command is currently a no-op and does not download or replace the binary. Upgrade by running the installation script again or installing a specific version.
