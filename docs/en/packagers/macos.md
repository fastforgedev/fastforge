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

The contents of `macos/packaging/dmg/` (background, icons, and so on) are copied next to the `.app`. If `macos/packaging/dmg/make_config.yaml` exists, it is used as an appdmg-style specification, with paths relative to that directory. The keys are `title` and `contents` (both required), `icon`, `background`, `background-color`, `icon-size`, `window` (`position`, `size`), `format` (default `UDZO`), `filesystem` (`HFS+` or `APFS`), and `code-sign` (`signing-identity`, optional `identifier`), which signs the DMG with `codesign`. Unknown keys fail packaging; the older appdmg format with `app` and `icons` is also accepted. Otherwise Fastforge uses a default layout with the app and an `/Applications` link, plus `background.png` when present.

Additional steps such as notarization can be run through a post-package hook (`--hook-post`, or `package.hooks.post` in a release job).

## PKG

PKG is the macOS installer package format. Fastforge builds it with `xcrun productbuild` and `pkgutil`. The PKG packager reads this optional configuration file:

```text
macos/packaging/pkg/make_config.yaml
```

| Key             | Description                                                  |
| --------------- | ------------------------------------------------------------ |
| `install-path`  | Installation location; defaults to `/Applications/`          |
| `sign-identity` | Installer signing identity, applied with `xcrun productsign` |
| `scripts`       | Directory containing pre/post-install scripts                |

If the file is missing, defaults are used; if it cannot be parsed, packaging fails.

```bash
fastforge package --targets pkg
```

To upload to the App Store:

```bash
fastforge publish --path dist/<version>/<artifact>.pkg --targets appstore
```

## ZIP

ZIP compresses a macOS `.app` bundle with `7z` into an archive suitable for downloads and publishing.

```bash
fastforge package --platform macos --targets zip
```

The resulting archive can be published directly to GitHub Releases, S3-compatible storage, or a custom download service:

```bash
fastforge publish --path dist/<version>/<artifact>.zip --targets github \
  --publish-arg repo=owner/repository \
  --publish-arg release-tag=v1.0.0
```

Return to the [packager overview](README.md).
