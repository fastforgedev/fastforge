# Packaging

English | [简体中文](../zh-Hans/packaging.md)

`fastforge package` prepares the project, runs the build, and turns its output into a distributable format.

```bash
fastforge package --targets apk,aab
fastforge package --platform macos --targets dmg,zip
```

`--targets` (alias `--target`) takes one or more comma-separated formats. `--platform` is optional: formats that belong to a single platform resolve it (`apk` → `android`, `dmg` → `macos`), multiple targets use the platform they share, and ambiguous formats (`zip`, `direct`, `custom`) fall back to the project's platform directories and the current host. See the [packager overview](packagers/README.md) for platform formats and environment requirements.

## Packaging Process

A packaging run contains these stages:

1. Detect the build system from project files.
2. Invoke the appropriate platform tools to build the project.
3. Run the pre-package hook.
4. Produce the final distributable artifact.
5. Run the post-package hook.

## Routing and Support

Routing depends on `pubspec.yaml` in the project root and the platform:

| Build system    | Project type            | Platform    | Formats                                                 |
| --------------- | ----------------------- | ----------- | ------------------------------------------------------- |
| Gradle          | Native Android          | Android     | `apk`, `aab`                                            |
| Xcode           | Native iOS              | iOS         | `ipa`                                                   |
| Xcode           | Native macOS            | macOS       | `dmg`, `pkg`, `zip`, `custom`                           |
| Flutter Builder | Contains `pubspec.yaml` | Android     | `apk`, `aab`, `custom`                                  |
| Flutter Builder | Contains `pubspec.yaml` | iOS         | `ipa`, `custom`                                         |
| Flutter Builder | Contains `pubspec.yaml` | macOS       | `dmg`, `pkg`, `zip`, `custom`                           |
| Flutter Builder | Contains `pubspec.yaml` | Windows     | `exe`, `msix`, `zip`, `direct`, `custom`                |
| Flutter Builder | Contains `pubspec.yaml` | Linux       | `appimage`, `deb`, `rpm`, `pacman`, `zip`, `direct`, `custom` |
| Flutter Builder | Contains `pubspec.yaml` | Web         | `zip`, `direct`, `custom`                               |
| Flutter Builder | Contains `pubspec.yaml` | OpenHarmony | `hap`, `app`, `custom`                                  |

Projects without `pubspec.yaml` are supported only for `android`, `ios`, and `macos`. All combinations are available from both the CLI and the `fastforge/package` action.

In Flutter projects, an unsupported platform/format pair is rejected before building, `flutter clean` runs at most once per invocation, and every platform except Android builds once and reuses the output for all targets.

> [!IMPORTANT]
> Host restrictions: iOS and macOS build only on macOS, Windows only on Windows, and Linux only on Linux. In a Flutter project, a target whose builder cannot run on the current host is **skipped with a warning and the command still exits with status 0**, so check the output rather than relying on the exit code alone.

Native projects differ: each target triggers its own build, `flutter clean` is not run, channel and flavor are not used in artifact names, and native macOS validates the format only after the Xcode build. See [Gradle Builder](builders/gradle.md) and [Xcode Builder](builders/xcode.md) for their limitations.

To inspect a Flutter Builder result separately, run `fastforge build`; see [Building](building.md).

## Packaging Options

| Option                                | Description                                                                 |
| ------------------------------------- | --------------------------------------------------------------------------- |
| `--channel <CHANNEL>`                 | Channel name; replaces the flavor segment of the default artifact name      |
| `--artifact-name <TEMPLATE>`          | Mustache artifact-name template (see below)                                 |
| `--skip-clean`                        | Skip `flutter clean` before building                                        |
| `--build-target <PATH>`               | Custom Flutter entry point                                                  |
| `--build-flavor <FLAVOR>`             | Flavor (also used by native Gradle projects)                                |
| `--build-target-platform <PLATFORM>`  | Target architectures                                                        |
| `--build-export-options-plist <PATH>` | iOS export options                                                          |
| `--build-dart-define <KEY=VALUE>`     | Compile-time variable; repeatable                                           |
| `--flutter-build-args <ARG,...>`      | Other build arguments: `flag` or `key=value`, comma-separated               |
| `--hook-pre` / `--hook-post`          | Shell commands run before / after the packager                              |

Arguments without a dedicated flag, such as `profile`, `obfuscate`, `split-debug-info=<dir>`, or `export-method=app-store`, can be passed through `--flutter-build-args`.

### Output Location and Artifact Names

Artifacts are written to `<output>/<version>/<artifact name>`, for example `dist/1.2.3+4/my_app-1.2.3+4-macos.dmg`. `<output>` is the `--output` argument when given, the `output` key of `distribute_options.yaml` otherwise, and `dist/` when neither is set — so a workflow can drive it without a `distribute_options.yaml`.

The default artifact name template is:

```text
{{name}}{{#flavor}}-{{flavor}}{{/flavor}}-{{build_name}}{{#has_build_number}}+{{build_number}}{{/has_build_number}}{{#is_profile}}-{{build_mode}}{{/is_profile}}-{{platform}}{{#is_installer}}-setup{{/is_installer}}{{#ext}}.{{ext}}{{/ext}}
```

When a channel is set, `{{channel}}` replaces the flavor segment. Available variables for `--artifact-name`: `name`, `version`, `build_name`, `build_number`, `build_mode`, `platform`, `flavor`, `channel`, `ext`, and the booleans `is_installer` (only `exe`), `is_profile`, and `has_build_number`.

### Format Configuration

Most packagers read an optional `<platform>/packaging/<format>/make_config.yaml`, such as `macos/packaging/dmg/make_config.yaml` or `linux/packaging/deb/make_config.yaml`. A missing file means defaults; a file that cannot be parsed fails packaging. Only the `custom` format requires its configuration file.

## Custom Format

The `custom` target runs your own script to produce the artifact. It requires `<platform>/packaging/custom/make_config.yaml`:

```yaml
script: ./scripts/package.sh # required
# Omit or leave empty to produce a directory artifact instead of a file.
output_extension: tar.gz
```

```bash
fastforge package --platform linux --targets custom
```

The script runs through `sh -c` (`cmd /c` on Windows) and receives `APP_NAME`, `APP_VERSION`, `BUILD_NAME`, `BUILD_NUMBER` (when present), `BUILD_MODE`, `FLAVOR` (when present), `CHANNEL` (when present), `BUILD_OUTPUT_DIRECTORY`, `OUTPUT_DIRECTORY`, and `OUTPUT_ARTIFACT_PATH`. It must create the artifact at `OUTPUT_ARTIFACT_PATH`; a nonzero exit status or a missing artifact fails packaging. Native iOS and Android projects do not support `custom`.

## Lifecycle Hooks

```bash
fastforge package --targets zip \
  --hook-pre './scripts/before-package.sh' \
  --hook-post './scripts/after-package.sh'
```

Hooks run as `sh -c <command>` on every host, so Windows requires `sh` on `PATH`. In addition to the environment and `distribute_options.yaml` variables, Fastforge provides:

- `PLATFORM`
- `PACKAGE_FORMAT`
- `BUILD_MODE`
- `OUTPUT_DIRECTORY`
- `BUILD_OUTPUT_DIRECTORY`
- `BUILD_OUTPUT_FILES` (colon-separated; empty for directory builds such as Windows, Linux, and Web)

Hooks do not receive `CHANNEL`, `FLAVOR`, or the artifact path. Packaging fails immediately if any hook exits with a nonzero status.

## Automation

Use [Local Workflows](workflows.md) to combine multiple packaging targets, publishing operations, or other commands. The `fastforge/package` action requires `platform` and a single `target`, and accepts `output` (default `dist/`), `artifact-name`, `channel`, `skip-clean`, `build-target`, `hook-pre`, `hook-post`, and `build-args` (a JSON object string for all other build arguments, such as `{"flavor":"dev","dart-define":{"APP_ENV":"dev"}}`).
