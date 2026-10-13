# dmg_maker

[English](README.md) | [中文](README-ZH.md)

A Rust DMG packager with basic compatibility for `node-appdmg`-style configuration files.

## Status

Work in progress (WIP).

Implemented so far:
- Standard config parsing
- Legacy config conversion (`icons/app/extra`)
- Core DMG pipeline (`hdiutil create/attach/convert/detach`)
- `.DS_Store` generation (based on `node-ds-store`)
- Retina background composition (`@2x` + `tiffutil`)
- Optional `codesign`

## Config Compatibility

Supported fields:
- `title` (required)
- `icon`
- `background`
- `background-color`
- `icon-size`
- `window.position`
- `window.size`
- `format` (`UDRW`/`UDRO`/`UDCO`/`UDZO`/`ULFO`/`ULMO`/`UDBZ`)
- `filesystem` (`HFS+`/`APFS`)
- `contents` (`type: link | file | position`)
- `code-sign.signing-identity`
- `code-sign.identifier`
- `code-sign.timestamp` (`true` adds `codesign --timestamp`)

Default values:
- `format`: `UDZO`
- `filesystem`: `HFS+`
- `icon-size`: `80`
- Window size: background image size if available, otherwise `640x480`

Background behavior:
- When `background` is set, `dmg_maker` generates a Finder alias record for `.background/<image>` in `.DS_Store` (compatible with `node-ds-store` alias data format).
- If alias generation fails, build fails with an error instead of silently falling back to plain color background.

## Usage

```bash
cargo run -p dmg_maker -- <json-path> <dmg-path>
```

Example:

```bash
cargo run -p dmg_maker -- \
  crates/dmg_maker/examples/standard.json \
  crates/dmg_maker/examples/sample.dmg
```

Help:

```bash
cargo run -p dmg_maker -- --help
```

Build all example DMGs:

```bash
./crates/dmg_maker/examples/build_examples.sh
```

## Test Example Files

Located at:
- `crates/dmg_maker/examples/standard.json`
- `crates/dmg_maker/examples/background_color.json`
- `crates/dmg_maker/examples/window.json`
- `crates/dmg_maker/examples/format_filesystem.json`
- `crates/dmg_maker/examples/compat_legacy.json`
- `crates/dmg_maker/examples/build_examples.sh`

Run tests:

```bash
cargo test -p dmg_maker
```

## Known Issues

- In some environments, parsing `hdiutil attach` output may fail and stop the build early.
- On this kind of failure, the target file is now preserved (it may be an empty file) for debugging.

## Runtime Tooling Requirements

This project depends on macOS system commands:
- `hdiutil`
- `cp`
- `du`
- `sips`
- `plutil`
- `tiffutil` (when retina background is used)
- `bless` (non-APFS only)
- `codesign` (when `code-sign` is configured)
- `xattr`
