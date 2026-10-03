---
name: fastforge-package
description: >-
  Build and package apps with Fastforge: APK/AAB for Android, IPA for iOS,
  DMG/PKG/ZIP for macOS, EXE/MSIX for Windows, AppImage/DEB/RPM/Pacman for
  Linux, HAP/APP for OpenHarmony, web bundles, and script-driven custom
  packaging. Use this skill whenever the user wants to build, package, or
  "打包" an app with fastforge — "package my app as a dmg", "build an apk with
  the dev flavor", "create an installer", "fastforge package fails" — even if
  they don't name a command. Covers fastforge build, fastforge package, the
  fastforge/package workflow action, builder routing (Gradle/Xcode/Flutter),
  flavors, channels, artifact-name templates, dart-define, hooks, make_config
  files, and artifact locations. Not for uploading artifacts
  (fastforge-publish) or store review/tracks (fastforge-stores).
---

# Fastforge Package

Turn a project into a distributable artifact.

## Step 1 — Identify the project type

`fastforge package` (and the `fastforge/package` action) route by
`pubspec.yaml` in the project root plus the platform:

1. No `pubspec.yaml`, platform `macos`/`ios` → **Xcode Builder**
2. No `pubspec.yaml`, platform `android` → **Gradle Builder**
3. No `pubspec.yaml`, any other platform → error (must be a Flutter project)
4. `pubspec.yaml` present (any Flutter project) → **Flutter Builder**

`fastforge build` does no routing — it always runs Flutter Builder.

Always run fastforge from the actual project root — detection is intentionally
simple and a wrong cwd selects the wrong builder.

## Step 2 — Know the format matrix

`fastforge package` routes every platform/format pair; an unsupported pair
fails fast **before** building:

| Platform | Targets |
| --- | --- |
| `android` | `aab`, `apk` |
| `ios` | `ipa` |
| `macos` | `dmg`, `pkg`, `zip` |
| `windows` | `exe`, `msix`, `zip`, `direct` |
| `linux` | `appimage`, `deb`, `rpm`, `pacman`, `zip`, `direct` |
| `web` | `zip`, `direct` |
| `ohos` | `hap`, `app` |
| every platform | `custom` (script-driven — see [references/other-platforms.md](references/other-platforms.md)) |

`direct` copies the raw build output without wrapping it in an archive or
installer. Host restrictions still apply: iOS/macOS need macOS, Windows needs
Windows, Linux needs Linux. **Caveat:** in a Flutter project, a target whose
builder can't run on this host is *skipped with a warning* and the command
still exits 0 (e.g. `--targets dmg` on Linux produces nothing) — check the
output, not just the exit code.

Native projects (no `pubspec.yaml`) differ: native iOS/Android reject
`custom`; native macOS builds before checking the target and rebuilds per
target. Xcode needs `project`/`scheme`, which have no dedicated CLI flags —
use a workflow `fastforge/package` action with `build-args`, or pass string
values via `--flutter-build-args project=…,scheme=…`
([references/ios.md](references/ios.md), [references/macos.md](references/macos.md)).

## Step 3 — Run it

`--platform` is optional: unambiguous targets resolve it alone (`apk` →
android, `dmg` → macos), while ambiguous ones (`zip`, `direct`, `custom`) fall
back to project layout and host OS. `--targets` takes a comma-separated list
(`--target` is an alias); in Flutter projects non-Android platforms build once
and reuse the output across targets, and `flutter clean` runs at most once per
invocation (skip it with `--skip-clean`).

```bash
fastforge package --targets apk,aab                      # platform inferred: android
fastforge package --platform macos --targets dmg,zip     # multi-target, one build
fastforge build --platform android --target aab          # raw artifact only, no packaging
```

Useful options:

- `--channel <name>` — distribution channel; in the default artifact name it
  replaces the flavor segment, and `custom` scripts get it as `CHANNEL` (hooks
  do not). Ignored for native projects.
- `--output <dir>` — directory the artifacts are written to; wins over the
  `output` key of `distribute_options.yaml`, which defaults to `dist/`.
- `--artifact-name <template>` — mustache template with `{{name}}`,
  `{{version}}`, `{{build_name}}`, `{{build_number}}`, `{{build_mode}}`,
  `{{platform}}`, `{{flavor}}`, `{{channel}}`, `{{ext}}`, booleans
  `{{is_installer}}`/`{{is_profile}}`/`{{has_build_number}}`, and sections like
  `{{#flavor}}-{{flavor}}{{/flavor}}`. Default:
  `{{name}}[-{{flavor}}]-{{build_name}}[+{{build_number}}][-profile]-{{platform}}[-setup].{{ext}}`.
- `--build-flavor`, `--build-target-platform`, repeatable
  `--build-dart-define KEY=VALUE`, `--build-export-options-plist`,
  `--flutter-build-args a,b=c` (any other key, e.g. `profile`,
  `export-method=app-store`), `--build-target lib/main_prod.dart`.

Each packaged target prints a JSON result summary. Artifacts land in
`<output>/<version>/<artifact name>`, e.g.
`dist/1.2.3+4/my_app-1.2.3+4-macos.dmg`, where `<output>` is `--output`, else
`output` from `distribute_options.yaml`, else `dist/` — a workflow can drive it
without a `distribute_options.yaml`.

When to write a workflow instead of running the CLI: repeatable releases,
flavor/target matrices, CI, native Xcode projects, or the user says they'll
need it again. Generate `.fastforge/workflows/<name>.yml`, validate with
`fastforge workflow validate`, and leave the file as a deliverable — read
[../fastforge/references/workflow.md](../fastforge/references/workflow.md)
first. The package action is narrower than the CLI: `platform` and `target`
are required (one target per step, no inference); the other inputs are
`output` (default `dist/`), `artifact-name`, `channel`, `skip-clean`,
`build-target`, `hook-pre`, `hook-post`, and `build-args` — a JSON object
string that carries everything else (`flavor`, `dart-define` as an object,
Xcode/Gradle arguments).

Platform specifics (Gradle tasks and flavors, Xcode two-stage IPA export,
make_config files, artifact search paths):

- Android → [references/android.md](references/android.md)
- iOS → [references/ios.md](references/ios.md)
- macOS → [references/macos.md](references/macos.md)
- Windows / Linux / Web / OpenHarmony / custom → [references/other-platforms.md](references/other-platforms.md)

## The packaging lifecycle

`fastforge package` runs: detect build system → build → `hook-pre` → produce
artifact → `hook-post`. Hooks run as `sh -c <command>` on every host (Windows
needs `sh` on `PATH`); a nonzero hook exit fails the run immediately. Hooks
receive the environment plus `PLATFORM`, `PACKAGE_FORMAT`, `BUILD_MODE`,
`OUTPUT_DIRECTORY`, `BUILD_OUTPUT_DIRECTORY`, and `BUILD_OUTPUT_FILES`
(colon-separated; empty for directory builds on Windows/Linux/Web) — the
natural place for signing, notarization, or artifact verification:

```bash
fastforge package --targets zip \
  --hook-pre './scripts/before.sh' --hook-post './scripts/after.sh'
```

`fastforge build` (raw artifact, no packaging step, no clean unless `--clean`)
prints a JSON result (`config`, `outputDirectory`, `outputFiles`,
`duration`). A build that succeeds but yields no artifact in the expected
directory is reported as a **failure** — deliberate, so later steps never
consume an empty directory.

## Format configuration (`make_config.yaml`)

Packagers read an optional config file at
`<platform>/packaging/<format>/make_config.yaml` — e.g.
`macos/packaging/dmg/make_config.yaml` (appdmg-style layout),
`macos/packaging/pkg/make_config.yaml`, `linux/packaging/deb/make_config.yaml`,
`windows/packaging/msix/make_config.yaml`. A missing file means defaults; an
unparsable one fails. Only `custom` requires its file. Details per platform
in the references above.

## After packaging

- Inspect the artifact (size, tech stack, signing): `fastforge analyze` — see
  the `fastforge` skill.
- Upload it: the `fastforge-publish` skill (including `playstore`/`pgyer`
  targets).
- Review, TestFlight, staged rollouts, store metadata: the `fastforge-stores`
  skill.
