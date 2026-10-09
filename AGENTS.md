# 仓库协作指南

本文件适用于整个 Fastforge 仓库。`AGENTS.md` 是协作说明的唯一维护源，根目录的 `CLAUDE.md` 通过相对符号链接指向本文件。

## 项目背景

Fastforge 用于构建、打包和发布应用，目前正在将核心实现从 Dart 迁移到 Rust。Rust 原生 CLI 与现有 Dart CLI 并行存在；Dart 版本仍需维护，命令行接口及配置格式应尽量保持兼容。开始修改前，先确认需求针对哪套实现，不要默认需要同时重写两套代码。

## 目录职责

| 路径 | 职责 |
| --- | --- |
| `apps/cli/` | Rust 原生 CLI，Cargo 包名 `fastforge_cli`，二进制名 `fastforge` |
| `crates/core/` | Rust 核心类型与配置 |
| `crates/app_analyzer/`、`app_builder/`、`app_packager/`、`app_publisher/` | 应用分析、构建、打包和发布能力，均位于 `crates/` 下 |
| `crates/dmg_maker/` | macOS DMG 制作 |
| `crates/stores/` | App Store Connect、AppGallery Connect 和 Google Play 客户端 |
| `crates/studio-core/` | Studio 共享核心逻辑 |
| `apps/studio-cli/` | 本地 Studio 服务，二进制名 `fastforge-studio` |
| `apps/studio-api/` | Cloudflare Workers 上的 Rust API、D1 迁移及 OpenAPI 契约 |
| `apps/studio-web/` | React / TanStack Start 前端 |
| `packages/studio-api-client/` | TypeScript API 客户端 |
| `packages/` 中的 Dart 包 | 现有 Dart CLI、构建器、打包器、发布器及辅助库 |
| `docs/en/`、`docs/zh-Hans/` | Rust 原生 CLI 的英文和简体中文文档 |
| `apps/docs/` | 现有文档网站（VitePress） |
| `examples/`、`fixtures/` | 使用示例与真实构建测试项目 |
| `scripts/generate/` | 商店客户端生成脚本与规范 |

## 环境与常用命令

以下命令默认在仓库根目录执行，按实际修改范围选择。以各目录的 README、清单文件和 CI 配置为准。

### Rust

使用支持 Rust 2024 edition 的 stable 工具链。原生 workspace 默认成员不包含 `studio-api`，该 crate 必须单独按 WASM 目标检查。

```sh
cargo run -p fastforge_cli -- --help
cargo fmt --all --check
cargo check --workspace --exclude studio-api --all-targets
cargo clippy --workspace --exclude studio-api --all-targets -- -D warnings
cargo build --workspace --exclude studio-api --all-targets

# 按修改范围运行测试，例如构建器的单元测试：
cargo test -p fastforge_app_builder --lib

# Studio Worker（需要安装 wasm32-unknown-unknown target）：
cargo check -p studio-api --target wasm32-unknown-unknown
```

`crates/app_builder/tests/` 中的集成测试会实际调用 Flutter、Gradle 或 Xcode，部分需要平台 SDK、签名身份和匹配的操作系统。运行前阅读 `crates/app_builder/tests/README.md`；不要把缺少本机工具链造成的失败当成代码回归，也不要提交机器专属的签名配置。

### Studio 与文档网站

使用 pnpm；根目录指定 `pnpm@10.11.1`，Node.js 要求为 `>=22.16.0`。避免引入 npm 或 Yarn 锁文件。

```sh
pnpm install
pnpm studio:dev
pnpm studio:lint
pnpm studio:typecheck
pnpm studio:test
pnpm studio:build
pnpm --filter docs build
```

`studio:typecheck` 也会检查 Worker，因此需要 WASM target。仅修改前端时，可用 `pnpm --filter studio-web typecheck` 和 `pnpm --filter studio-web test` 缩小检查范围；共享包修改还需检查使用它们的应用。

### Dart / Flutter

使用 Flutter / Dart SDK 及根目录 `pubspec.yaml` 声明的 Melos 6。

```sh
dart pub get
dart run melos bootstrap
dart run melos run analyze
dart run melos run test
```

只改某个包时，可在该包目录运行 `flutter analyze --fatal-infos`，有测试时运行 `flutter test`。格式检查使用 `dart format --output=none --set-exit-if-changed <修改的文件或目录>`；现有 Melos `format-check` 脚本带有修改参数，不应视为只读检查。

## 修改约定

- 沿用相邻代码的结构和风格；在对应核心 crate 或共享包实现可复用逻辑，CLI 层负责参数及命令编排。
- 修改命令参数、配置解析或发布行为时，核对现有示例及两种语言的相关文档；涉及破坏性变更时明确说明兼容性影响。
- `distribute_options.yaml`（含 `release` 命令）已废弃：只为兼容 Dart 版保留现有行为和缺陷修复，不新增字段或功能；新配置放到 `.fastforge/`（项目）或 `~/.fastforge/`（本机），新的自动化能力放到工作流。
- Studio 的本地服务和 Worker 共享 `apps/studio-api/openapi.yaml` 契约。API 改动需核对两端实现、能力声明和客户端，保持行为一致。
- 修改 OpenAPI 后，运行 `pnpm --filter studio-api-client codegen` 更新类型，再运行 `pnpm --filter studio-api-client codegen:check` 验证。不要直接手改生成的 `src/schema.d.ts`；其他生成代码也应优先修改生成来源。
- Studio 前端直接使用 `@dazzlabs/dazzui` 组件和样式；业务组件放在 `apps/studio-web/src/components/`，避免添加仅转发 DazzUI 的封装。
- 依赖调整使用对应生态的包管理工具，只保留与改动相关的锁文件变化。不要提交构建产物、缓存、令牌、签名材料或机器专属配置。
- 行为变更补充能覆盖实际回归的测试；纯文档改动只需检查内容、路径及差异，无需运行整套构建。未能执行的检查应注明原因，不声称通过。
- 完成时简要说明修改内容、实际执行的验证和仍存在的限制。
