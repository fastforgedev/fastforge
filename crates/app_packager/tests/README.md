# Native packaging integration tests

`linux_packagers.rs` and `windows_native.rs` exercise the real system packaging
tools and package managers with small native test bundles. Missing tools can
skip a Linux check locally; `FASTFORGE_REQUIRE_NATIVE_TESTS=1` makes that a
failure in CI. Linux package manager tests use temporary roots, never the
host's installed packages. Windows installer tests require the signing and
installation setup in `.github/workflows/ci.yml`.

`flutter_desktop.rs` covers the real Flutter build → native packager → launch
path. Each test copies `fixtures/flutter_app` into a fresh temporary project,
builds release versions 1.0.0 and 1.1.0 with `FlutterAppBuilder`, and packages
each version. The dedicated `lib/desktop_smoke.dart` entry point starts the
normal fixture UI and exits after the first rendered frame, printing the
compiled version. A successful launch therefore checks the engine, AOT code,
assets and upgraded application, rather than just checking a process exists.
Each launch has a 60-second timeout and fails on a missing version marker.

Requirements: Flutter 3.47.6 stable with its bundled Dart 3.13.5, matching
`.github/workflows/ci.yml`, the host's desktop build
toolchain, a graphical session, and several GiB free for build intermediates
(including macOS framework debug symbols). No Apple Developer account, distribution
certificate or notarization credentials are used.

## Linux AppImage

Install `appimagetool`, `ldd`, GTK development libraries, clang, CMake, Ninja
and pkg-config. The test inspects the extracted AppDir, installs by replacing
one AppImage path, and launches both versions with
`--appimage-extract-and-run` (no FUSE mount required).

```sh
cargo test -p fastforge_app_packager --test flutter_desktop appimage_flutter -- --ignored --nocapture --test-threads=1
```

For headless CI, install Xvfb and Mesa, then run:

```sh
GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s '-screen 0 1280x800x24' cargo test -p fastforge_app_packager --test flutter_desktop appimage_flutter -- --ignored --nocapture --test-threads=1
```

CI installs appimagetool 1.9.1. Its wrapper uses `--appimage-extract-and-run`
so the tool itself also works without FUSE. CI fetches the Type 2 runtime
explicitly with a download timeout and passes `--runtime-file`, avoiding an
unbounded implicit download inside appimagetool.

## macOS DMG and PKG

Install Xcode and its command-line tools. The test creates and mounts real
DMGs, launches their applications, copies them into a temporary Applications
directory, replaces version 1 with version 2 and launches again. It expands
both PKGs with `pkgutil --expand-full`, checks domain and relocation metadata,
and launches the payload applications.

```sh
cargo test -p fastforge_app_packager --test flutter_desktop macos_flutter -- --ignored --nocapture --test-threads=1
```

For actual PKG installation and upgrade, set `FASTFORGE_TEST_PKG_INSTALL=1`.
This requires passwordless `sudo` and fails if it is unavailable. It creates
a separate 1 GiB HFS+ disk image, runs `installer` with that volume as its
target, launches both installed versions, and checks application versions
and receipts on that volume. The mounted images and temporary project are
cleaned up when the test finishes. The host's `/Applications` is untouched.

```sh
FASTFORGE_TEST_PKG_INSTALL=1 cargo test -p fastforge_app_packager --test flutter_desktop macos_flutter -- --ignored --nocapture --test-threads=1
```

The `flutter-desktop` CI job runs Linux and Windows real-build checks, the
AppImage launch/upgrade test, and the macOS test with PKG installation enabled.
Unsigned/ad hoc package validation does not cover Gatekeeper, notarization,
distribution signing or an automatic update service.
