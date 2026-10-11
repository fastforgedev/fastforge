# Build layer integration tests

`flutter_builder.rs`, `gradle_builder.rs`, and `xcode_builder.rs` run the real
`flutter`, `gradle`, and `xcodebuild` CLIs against fixture projects under
[`fixtures/`](../../../fixtures) at the repo root, and assert on real build
artifacts. They complement — not replace — the synthetic-`BuildConfig` unit
tests already inline in `src/gradle/mod.rs` and `src/xcode/{ios,macos}.rs`,
which cover argument encoding and output-path logic without touching a real
toolchain.

## Fixtures used

| Test file            | Fixture(s)                                             |
| --------------------- | ------------------------------------------------------- |
| `flutter_builder.rs`  | `fixtures/flutter_app`                                   |
| `gradle_builder.rs`   | `fixtures/native_android`, `fixtures/native_android_flavors` |
| `xcode_builder.rs`    | `fixtures/native_ios`, `fixtures/native_macos`            |

## Requirements to run

- **flutter_builder.rs** — Flutter 3.47.6 stable with its bundled Dart 3.13.5
  on `PATH`, matching `.github/workflows/ci.yml`
  (or pass `FLUTTER_ROOT` via the `environment` argument). The macOS fixture
  uses local ad hoc signing; `builds_ios` needs a usable Xcode development
  signing identity (an Apple ID signed into Xcode). The
  `builds_linux`/`builds_windows` tests are `#[ignore]`d — they only build on
  their respective host OS and run in the `flutter-desktop` CI matrix. They
  validate the bundle directory's executable, engine, AOT code and assets;
  `output_files` is intentionally empty for these directory-style builders.
- **gradle_builder.rs** — `gradle` on `PATH` (no wrapper is checked into the
  fixtures — `GradleAppBuilder` falls back to the system `gradle`) and
  `ANDROID_HOME` or `ANDROID_SDK_ROOT` set in the environment. Gradle does
  **not** read `HTTP_PROXY`/`HTTPS_PROXY`; if your network needs a proxy,
  export `GRADLE_OPTS` before running, e.g.:
  ```sh
  export GRADLE_OPTS="-Dhttps.proxyHost=127.0.0.1 -Dhttps.proxyPort=7890 -Dhttp.proxyHost=127.0.0.1 -Dhttp.proxyPort=7890"
  ```
- **xcode_builder.rs** — macOS with the Xcode command-line tools.
  `native_macos` builds unsigned (`CODE_SIGNING_ALLOWED=NO`) and needs no
  Apple account. `native_ios` needs a working development signing
  identity/provisioning profile — see the caveat below.

## Running

Every test that changes the process's working directory is `#[serial]`-tagged
(see `support/mod.rs`), so plain parallel `cargo test` is safe:

```sh
cargo test -p fastforge_app_builder
```

Run a desktop build explicitly on a matching machine:

```sh
cargo test -p fastforge_app_builder --test flutter_builder builds_linux -- --ignored --nocapture
# On Windows, use builds_windows instead.
```

Real DMG, PKG and AppImage packaging/launch/upgrade tests build two versions
of the same fixture through `FlutterAppBuilder`; see
[`app_packager/tests/README.md`](../../app_packager/tests/README.md).

## Known limitation: iOS signing is machine-specific

`fixtures/native_ios/project.yml` hardcodes `DEVELOPMENT_TEAM` to the Apple
Developer team ID of whichever machine first generated the fixture. On a
different machine/Apple ID, `builds_ios_ipa` in `xcode_builder.rs` will fail
to find a matching automatic-signing identity. To run it locally, edit
`DEVELOPMENT_TEAM` in `project.yml` to your own team ID and regenerate (see
below) — do not commit that change.

## Regenerating the Xcode fixtures

`fixtures/native_ios` and `fixtures/native_macos` are generated from
`project.yml` via [XcodeGen](https://github.com/yonaskolb/XcodeGen)
(`brew install xcodegen`). After editing `project.yml`, regenerate the
`.xcodeproj` from inside the fixture directory:

```sh
xcodegen generate
```
