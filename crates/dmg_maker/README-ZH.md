# dmg_maker

[English](README.md) | [中文](README-ZH.md)

Rust 实现的 DMG 打包器，兼容基础 `node-appdmg` 风格配置。

## 状态

开发中（WIP）。

已完成：
- 标准配置解析
- legacy 配置转换（`icons/app/extra`）
- DMG 生成主流程（`hdiutil create/attach/convert/detach`）
- `.DS_Store` 生成（参考 `node-ds-store`）
- retina 背景图拼接（`@2x` + `tiffutil`）
- 可选 `codesign`

## 配置兼容性

支持字段：
- `title` (required)
- `icon`
- `background`
- `background-color`
- `icon-size`
- `window.position`
- `window.size`
- `format` (`UDRW`/`UDRO`/`UDCO`/`UDZO`/`ULFO`/`ULMO`/`UDBZ`)
- `filesystem` (`HFS+`/`APFS`)
- `contents`（`type: link | file | position`）
- `code-sign.signing-identity`
- `code-sign.identifier`
- `code-sign.timestamp`（为 `true` 时追加 `codesign --timestamp`）

默认值：
- `format`: `UDZO`
- `filesystem`: `HFS+`
- `icon-size`: `80`
- window size: 背景图尺寸，否则 `640x480`

背景图行为：
- 配置了 `background` 时，`dmg_maker` 会在 `.DS_Store` 中为 `.background/<image>` 写入 Finder alias record（与 `node-ds-store` 的 alias 数据格式兼容）。
- 如果 alias 生成失败，构建会直接报错，不再静默回退到纯色背景。

## 使用方式

```bash
cargo run -p dmg_maker -- <json-path> <dmg-path>
```

示例：

```bash
cargo run -p dmg_maker -- \
  crates/dmg_maker/examples/standard.json \
  crates/dmg_maker/examples/sample.dmg
```

查看帮助：

```bash
cargo run -p dmg_maker -- --help
```

一次构建所有示例 DMG：

```bash
./crates/dmg_maker/examples/build_examples.sh
```

## 测试示例文件

位于：
- `crates/dmg_maker/examples/standard.json`
- `crates/dmg_maker/examples/background_color.json`
- `crates/dmg_maker/examples/window.json`
- `crates/dmg_maker/examples/format_filesystem.json`
- `crates/dmg_maker/examples/compat_legacy.json`
- `crates/dmg_maker/examples/build_examples.sh`

运行测试：

```bash
cargo test -p dmg_maker
```

## 已知问题

- 某些环境下 `hdiutil attach` 输出解析可能失败，导致构建提前终止。
- 这类失败时现在会保留目标文件（可能是空文件），方便排查。

## 依赖系统工具

运行时依赖 macOS 系统命令：
- `hdiutil`
- `cp`
- `du`
- `sips`
- `plutil`
- `tiffutil`（有 retina 背景时）
- `bless`（非 APFS）
- `codesign`（配置了 `code-sign` 时）
- `xattr`
