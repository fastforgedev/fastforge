# macOS

English | [简体中文](../../zh-Hans/packagers/macos.md)

Fastforge packages macOS applications as [DMG](#dmg), [PKG](#pkg), or [ZIP](#zip).

## Current Status

| Build system    | `package` status                                              |
| --------------- | ------------------------------------------------------------- |
| Xcode           | DMG, PKG, and ZIP; package action recommended                 |
| Flutter Builder | DMG, PKG, and ZIP connected through CLI/action                |

Fastforge selects Xcode Builder or Flutter Builder from the project files, then passes the generated `.app` to the matching packager. Flutter projects can use `fastforge package` directly and build once for several formats:

```bash
fastforge package --targets dmg,pkg,zip
```

Xcode projects should pass project arguments through workflow `build-args`; see [Xcode Builder](../builders/xcode.md#macos). Artifacts are written to `dist/<version>/`.

## Requirements

- macOS
- Xcode command-line tools
- SDKs and build tools required by the project
- `7z` on `PATH` for ZIP

## DMG

DMG is a common disk image distribution format for macOS. The current release uses the built-in DMG maker and no longer requires the globally installed `appdmg` Node.js tool.

```bash
fastforge package --targets dmg
```

The contents of `macos/packaging/dmg/` (background, icons, and so on) are copied next to the `.app`. If `macos/packaging/dmg/make_config.yaml` exists, it is used as an appdmg-style specification, with paths relative to that directory. The keys are `title` and `contents` (both required), `icon`, `background`, `background-color`, `icon-size`, `window` (`position`, `size`), `format` (default `UDZO`), `filesystem` (`HFS+` or `APFS`), and `code-sign` (`signing-identity`, optional `identifier` and `timestamp`), which signs the DMG with `codesign`. Fastforge also reads `notarize` (see [Notarization](#notarization)). Unknown keys fail packaging; the older appdmg format with `app` and `icons` is also accepted. Otherwise Fastforge uses a default layout with the app and an `/Applications` link, plus `background.png` when present.

```yaml
title: Hello World            # volume name shown when mounted
icon: volume.icns             # volume icon
background: background.png    # or background-color: "#f5f5f7"
icon-size: 96
window:
  position: { x: 200, y: 120 }
  size: { width: 640, height: 480 }
format: ULFO                  # UDZO (default), UDRO, UDCO, UDBZ, ULFO, ULMO, UDRW
filesystem: APFS              # HFS+ (default) or APFS
contents:
  - { x: 448, y: 344, type: link, path: /Applications }
  - { x: 192, y: 344, type: file, path: Hello World.app }
  - { x: 100, y: 100, type: file, path: README.txt, name: Read Me.txt }
code-sign:
  signing-identity: "Developer ID Application: Example (TEAMID)"
```

In `contents`, `file` copies a file or bundle into the image (optionally renamed with `name`), `link` creates a symlink, and `position` only places an item that is already there. Coordinates are icon centers in points from the window's top-left corner. The `.app` entry must use the bundle's real name, which Xcode takes from `PRODUCT_NAME` and may differ from the `pubspec.yaml` name; packaging fails with the name of the built bundle otherwise.

To notarize the DMG, see [Notarization](#notarization). Other steps can be run through a post-package hook (`--hook-post`, or `package.hooks.post` in a release job).

## PKG

PKG is the macOS installer package format. Fastforge builds it with `xcrun productbuild` and `pkgutil`. The package installs on the startup disk only (`enable_localSystem`). The PKG packager reads this optional configuration file:

```text
macos/packaging/pkg/make_config.yaml
```

| Key             | Description                                                  |
| --------------- | ------------------------------------------------------------ |
| `install-path`  | Installation location; defaults to `/Applications/`          |
| `sign-identity` | Installer signing identity, applied with `xcrun productsign` |
| `scripts`       | Directory containing pre/post-install scripts                |
| `notarize`      | `true` notarizes and staples the package; see [Notarization](#notarization) |

If the file is missing, defaults are used; if it cannot be parsed, packaging fails.

```bash
fastforge package --targets pkg
```

To upload to the App Store:

```bash
fastforge publish --path dist/<version>/<artifact>.pkg --targets appstore
```

## ZIP

ZIP compresses a macOS `.app` bundle with `7z` into an archive suitable for downloads and publishing. An optional `macos/packaging/zip/make_config.yaml` accepts `notarize: true` (see [Notarization](#notarization)).

```bash
fastforge package --platform macos --targets zip
```

The resulting archive can be published directly to GitHub Releases, S3-compatible storage, or a custom download service:

```bash
fastforge publish --path dist/<version>/<artifact>.zip --targets github \
  --publish-arg repo=owner/repository \
  --publish-arg release-tag=v1.0.0
```

## Notarization

Apps distributed outside the Mac App Store must be signed with a Developer ID and notarized, or Gatekeeper blocks or warns on first launch. Fastforge can notarize DMG, PKG, and ZIP artifacts with `xcrun notarytool` and staple the tickets with `xcrun stapler`. It is off by default; turn it on for every macOS target with `--notarize`:

```bash
fastforge package --platform macos --targets dmg,zip --notarize
```

or for one format with `notarize: true` in its `macos/packaging/<dmg|pkg|zip>/make_config.yaml` (the ZIP packager reads only this key from its file). In a workflow, set the `fastforge/package` input `notarize: "true"`.

Fastforge does not sign the app: the `.app` from the build must already be signed with a "Developer ID Application" identity, with the hardened runtime and a secure timestamp (set the macOS Release signing in Xcode). This is checked before anything is submitted, so a missing requirement fails in seconds with a message instead of after a notary round-trip.

Credentials come from the environment, so CI needs no config changes. The first complete set is used:

| Credentials                    | Variables                                                                                     |
| ------------------------------ | --------------------------------------------------------------------------------------------- |
| Stored `notarytool` profile    | `NOTARYTOOL_KEYCHAIN_PROFILE` (saved with `xcrun notarytool store-credentials`)                |
| App Store Connect API key      | `APP_STORE_CONNECT_KEY_ID`, `APP_STORE_CONNECT_KEY_PATH` (`.p8`), `APP_STORE_CONNECT_ISSUER_ID` (omit for individual keys) |
| Apple ID                       | `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`                                     |

The API key variables are the same ones `fastforge publish --targets appstore` uses. With `--host`, the variables must be set on the remote host.

What each format does:

- **ZIP**: notarizes the `.app` (submitted as a zip made with `ditto`), staples it, then zips the stapled app. A ZIP cannot be stapled itself.
- **DMG**: notarizes and staples the `.app`, builds the image from the stapled app, signs the image with `--timestamp`, then notarizes and staples the DMG. Without `code-sign` in `make_config.yaml`, the image is signed with the app's Developer ID Application identity.
- **PKG**: needs `sign-identity` (a "Developer ID Installer" identity); signs with `productsign --timestamp`, then notarizes and staples the package.

The `.app` is stapled in place in the build output, so with `--targets dmg,zip` it is notarized once and both archives contain the stapled app. `notarytool submit --wait` usually returns within a few minutes, but each submission (app, DMG, PKG) adds to the packaging time. When a submission is `Invalid` or `Rejected`, Fastforge fetches the notary log, prints its issues, and fails packaging. Check the results with [`fastforge analyze`](../tools/analyze.md), which reports signing, stapling, and Gatekeeper state. Hooks still run as before for any custom steps.

Return to the [packager overview](README.md).
