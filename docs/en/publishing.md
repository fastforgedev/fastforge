# Publishing

English | [简体中文](../zh-Hans/publishing.md)

`publish` sends an existing file or directory to one or more publishing targets.

## Basic Usage

```bash
fastforge publish \
  --path dist/my-app.apk \
  --target fir
```

`--path` and `--targets` are required (`--target` and `-t` are aliases; separate multiple targets with commas). Pass publishing parameters with repeatable `--publish-arg KEY=VALUE` options:

```bash
fastforge publish \
  --path dist/app.zip \
  --target github \
  --publish-arg repo=owner/repository \
  --publish-arg release-title=v1.0.0 \
  --publish-arg release-tag=v1.0.0
```

`--app-version` provides the version used by targets that need one (for example the default GitHub release title); when omitted, the `version` in `./pubspec.yaml` is used if present. An explicit version (`--app-version` or an `app-version` argument) must be a semantic version such as `1.0.0` or `1.2.3+45`, otherwise publishing fails for every target.

The provider options of the Dart CLI are also accepted, such as `--github-repo`, `--firebase-app`, `--pgyer-password`, and `--playstore-track`. They are forwarded as `<target>-<name>` parameters; a `--publish-arg` with the same key takes precedence over the option. Any parameter key may use a `<target>-` prefix: for that target the prefix is stripped, and the prefixed key takes precedence over the unprefixed one. This is useful when one command publishes to several targets.

See the [publisher overview](publishers/README.md) for available targets, parameters, and credential requirements. You can also inspect the provider options in the `fastforge publish --help` output; the value placeholder of `--targets` there lists only some targets, and the overview is the complete list.

## Credentials

Prefer passing credentials through environment variables. Some publishers also accept credentials as parameters (for example `access-key` or `client-secret`), but that exposes them in command history, so avoid it and never commit secrets to the repository. `fastforge publish` reads the process environment plus the `variables` in `distribute_options.yaml` in the current directory (which take precedence). Each publisher page lists the environment variables that target reads.

## Multi-step Automation

Use [Local Workflows](workflows.md) to combine build, package, publish, and shell commands. The `fastforge/publish` action publishes to a single `target`. Put non-sensitive parameters either in a `publish-args` JSON string (string values only) or as bare `with` fields; when `publish-args` is present, other `with` fields are ignored. Actions read credentials from the environment of the `fastforge workflow run` process only.

The legacy `fastforge release` command runs the releases in `distribute_options.yaml`: each job packages its target and, when `publish_to` or `publish.target` is set, publishes the first artifact with the parameters in `publish.args`. The publisher sees the process environment plus the global, release, and job `variables`, and the `APPSTORE_*` / `APP_STORE_CONNECT_*` variables are also copied into the matching `appstore` arguments when `publish.args` does not set them. `release` does not pass an app version, so targets that need one fall back to `./pubspec.yaml`. This command is deprecated and kept only for compatibility; use workflows for new automation.
