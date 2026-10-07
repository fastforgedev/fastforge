# 打包

[English](../en/packaging.md) | 简体中文

`fastforge package` 负责准备项目、执行构建并把产物整理为可分发格式。

```bash
fastforge package --targets apk,aab
fastforge package --platform macos --targets dmg,zip
```

`--targets`（别名 `--target`）接受一个或多个以逗号分隔的格式。`--platform` 可省略：只属于单一平台的格式可以直接确定平台（`apk` → `android`、`dmg` → `macos`），多个 target 取它们共同的平台，有歧义的格式（`zip`、`direct`、`custom`）则根据项目的平台目录和当前宿主判断。各平台的格式和环境要求见[打包器总览](packagers/README.md)。

## 打包流程

一次打包包含以下阶段：

1. 根据项目文件识别构建系统。
2. 调用对应平台工具完成构建。
3. 运行 pre-package hook。
4. 生成最终分发产物。
5. 运行 post-package hook。

## 路由与支持范围

路由取决于项目根目录是否存在 `pubspec.yaml` 以及目标平台：

| 构建系统        | 适用项目          | 平台        | 格式                                                          |
| --------------- | ----------------- | ----------- | ------------------------------------------------------------- |
| Gradle          | 原生 Android      | Android     | `apk`、`aab`                                                  |
| Xcode           | 原生 iOS          | iOS         | `ipa`                                                         |
| Xcode           | 原生 macOS        | macOS       | `dmg`、`pkg`、`zip`、`custom`                                 |
| Flutter Builder | 含 `pubspec.yaml` | Android     | `apk`、`aab`、`custom`                                        |
| Flutter Builder | 含 `pubspec.yaml` | iOS         | `ipa`、`custom`                                               |
| Flutter Builder | 含 `pubspec.yaml` | macOS       | `dmg`、`pkg`、`zip`、`custom`                                 |
| Flutter Builder | 含 `pubspec.yaml` | Windows     | `exe`、`msix`、`zip`、`direct`、`custom`                      |
| Flutter Builder | 含 `pubspec.yaml` | Linux       | `appimage`、`deb`、`rpm`、`pacman`、`zip`、`direct`、`custom` |
| Flutter Builder | 含 `pubspec.yaml` | Web         | `zip`、`direct`、`custom`                                     |
| Flutter Builder | 含 `pubspec.yaml` | OpenHarmony | `hap`、`app`、`custom`                                        |

不含 `pubspec.yaml` 的项目只支持 `android`、`ios` 和 `macos`。以上组合都可以通过 CLI 和 `fastforge/package` action 使用。

在 Flutter 项目中，不支持的平台/格式组合会在构建前被拒绝；每次调用最多执行一次 `flutter clean`；除 Android 外，所有平台只构建一次并在多个 target 间复用构建结果。

> [!IMPORTANT]
> 宿主限制：iOS 和 macOS 只能在 macOS 上构建，Windows 只能在 Windows 上构建，Linux 只能在 Linux 上构建。在 Flutter 项目中，如果某个 target 的构建器无法在当前宿主运行，该 target 会**输出警告并被跳过，命令仍以状态码 0 退出**。请检查输出，不要只依赖退出码。

原生项目的行为有所不同：每个 target 单独构建，不执行 `flutter clean`，产物名不使用 channel 和 flavor，原生 macOS 会在 Xcode 构建完成后才校验格式。相关限制见 [Gradle Builder](builders/gradle.md) 和 [Xcode Builder](builders/xcode.md)。

需要单独检查 Flutter Builder 的结果时，运行 `fastforge build`，详见[构建](building.md)。

## 打包参数

| 参数                                  | 说明                                                   |
| ------------------------------------- | ------------------------------------------------------ |
| `--channel <CHANNEL>`                 | 渠道名；在默认产物名中替代 flavor 段                   |
| `--artifact-name <TEMPLATE>`          | Mustache 产物名模板（见下文）                          |
| `--skip-clean`                        | 构建前跳过 `flutter clean`                             |
| `--build-target <PATH>`               | 自定义 Flutter 入口                                    |
| `--build-flavor <FLAVOR>`             | flavor（原生 Gradle 项目同样使用）                     |
| `--build-target-platform <PLATFORM>`  | 目标架构                                               |
| `--build-export-options-plist <PATH>` | iOS 导出配置                                           |
| `--build-dart-define <KEY=VALUE>`     | 编译变量；可重复                                       |
| `--flutter-build-args <ARG,...>`      | 其他构建参数，`flag` 或 `key=value`，以逗号分隔        |
| `--hook-pre` / `--hook-post`          | 打包器执行前 / 后运行的 shell 命令                     |

没有专用参数的选项，例如 `profile`、`obfuscate`、`split-debug-info=<dir>` 或 `export-method=app-store`，可以通过 `--flutter-build-args` 传入。

### 输出位置与产物名

产物写入 `<output>/<version>/<产物名>`，例如 `dist/1.2.3+4/my_app-1.2.3+4-macos.dmg`。`<output>` 优先取 `--output` 参数，其次取 `distribute_options.yaml` 的 `output`，两者都没有时为 `dist/` —— 因此工作流无需 `distribute_options.yaml` 也能指定输出目录。

默认产物名模板为：

```text
{{name}}{{#flavor}}-{{flavor}}{{/flavor}}-{{build_name}}{{#has_build_number}}+{{build_number}}{{/has_build_number}}{{#is_profile}}-{{build_mode}}{{/is_profile}}-{{platform}}{{#is_installer}}-setup{{/is_installer}}{{#ext}}.{{ext}}{{/ext}}
```

设置 channel 后，`{{channel}}` 会替代 flavor 段。`--artifact-name` 可用的变量有 `name`、`version`、`build_name`、`build_number`、`build_mode`、`platform`、`flavor`、`channel`、`ext`，以及布尔值 `is_installer`（仅 `exe`）、`is_profile` 和 `has_build_number`。

### 格式配置

大多数打包器会读取可选的 `<platform>/packaging/<format>/make_config.yaml`，例如 `macos/packaging/dmg/make_config.yaml` 或 `linux/packaging/deb/make_config.yaml`。文件不存在时使用默认值；文件无法解析时打包失败。只有 `custom` 格式必须提供配置文件。

Linux 的 AppImage、DEB、RPM 和 Pacman 打包器还接受各格式自己的原始文件，例如 Debian 的 `control` 文件或 RPM spec，并用下文的打包变量渲染。原始文件优先于 `make_config.yaml`，详见 [Linux](packagers/linux.md#配置)。

### 打包变量

Fastforge 在原始打包文件中以 `${NAME}` 的形式提供以下变量，同时把它们设置到打包工具、`custom` 脚本和生命周期钩子的环境中；值为空的变量不会设置到环境中。

| 变量                                                      | 值                                                                                     |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `APP_NAME`、`APP_BINARY_NAME`                             | `pubspec.yaml` 中的名称和可执行文件名                                                  |
| `APP_VERSION`、`BUILD_NAME`、`BUILD_NUMBER`               | `1.2.3+4`、`1.2.3`、`4`；没有构建号时 `BUILD_NUMBER` 为空                               |
| `APP_DISPLAY_NAME`                                        | `project.display_name`，其次 `make_config.yaml` 的 `display_name`，最后为 `APP_NAME`    |
| `APP_DESCRIPTION`、`APP_HOMEPAGE`                         | `project.description` / `project.homepage`，其次取自 `pubspec.yaml`                     |
| `APP_ID`                                                  | `project.app_id`                                                                       |
| `BUILD_MODE`、`FLAVOR`、`CHANNEL`、`PLATFORM`、`PACKAGE_FORMAT` | 构建与打包选项                                                                    |
| `BUILD_OUTPUT_DIRECTORY`、`OUTPUT_DIRECTORY`              | 构建输出目录和产物输出目录的绝对路径                                                   |
| `PACKAGE_NAME`                                            | 仅 Linux。`make_config.yaml` 的 `package_name`，其次为该格式的默认值                    |
| `PACKAGE_ARCH`、`ARCH`                                    | 仅 Linux。该格式的架构名（DEB 为 `amd64`）以及 `x86_64` 或 `aarch64`                    |
| `INSTALL_DIR`                                             | DEB 和 Pacman 为 `/opt/<binary>`，RPM 为 `/usr/share/<name>`                           |
| `RPM_RELEASE`                                             | 仅 RPM。构建号的第一段，没有构建号时为 `1`                                              |
| `PACKAGING_DIRECTORY`、`OUTPUT_ARTIFACT_PATH`             | 仅 Linux。暂存目录（包根目录、RPM 构建目录或 AppDir）和产物路径                        |

项目元数据和自定义变量来自 `.fastforge/config.yaml`。`env:` 中整个值写成 `${NAME}` 时，从环境变量读取。`env:` 中的键不能与内置变量重名。

```yaml
project:
  display_name: Hello World
  description: A short description
  homepage: https://example.com
  app_id: com.example.hello
  icon: assets/logo.png
env:
  SUPPORT_EMAIL: team@example.com
  SIGNING_KEY_ID: ${SIGNING_KEY_ID}
```

## 自定义格式

`custom` target 运行你自己的脚本来生成产物，必须提供 `<platform>/packaging/custom/make_config.yaml`：

```yaml
script: ./scripts/package.sh # 必填
# 省略或留空时生成目录产物，而不是文件。
output_extension: tar.gz
```

```bash
fastforge package --platform linux --targets custom
```

脚本通过 `sh -c`（Windows 上为 `cmd /c`）执行，并获得适用于所有格式的[打包变量](#打包变量)（包括 `.fastforge/config.yaml` 中的 `env:`）以及 `OUTPUT_ARTIFACT_PATH`。`BUILD_NUMBER`、`FLAVOR` 和 `CHANNEL` 仅在存在时设置。脚本必须在 `OUTPUT_ARTIFACT_PATH` 生成产物；退出码非零或产物不存在时打包失败。原生 iOS 和 Android 项目不支持 `custom`。

## 生命周期钩子

```bash
fastforge package --targets zip \
  --hook-pre './scripts/before-package.sh' \
  --hook-post './scripts/after-package.sh'
```

钩子在所有宿主上都以 `sh -c <command>` 执行，因此 Windows 上需要 `PATH` 中有 `sh`。除环境变量和 `distribute_options.yaml` 中的变量外，Fastforge 还会提供：

- `PLATFORM`
- `PACKAGE_FORMAT`
- `BUILD_MODE`
- `OUTPUT_DIRECTORY`
- `BUILD_OUTPUT_DIRECTORY`
- `BUILD_OUTPUT_FILES`（以 `:` 分隔；Windows、Linux、Web 等目录型构建为空）

钩子还会获得适用于所有格式的[打包变量](#打包变量)，例如 `APP_VERSION`、`APP_DISPLAY_NAME`、`CHANNEL`、`FLAVOR` 以及 `.fastforge/config.yaml` 中的 `env:`；只有配置了钩子时才会读取 `.fastforge/config.yaml`。钩子不会获得产物路径。任意钩子返回非零退出码时，打包立即失败。

## 自动化

需要组合多个打包目标、发布或其他命令时，使用[本地工作流](workflows.md)。`fastforge/package` action 要求提供 `platform` 和单个 `target`，并接受 `output`（默认 `dist/`）、`artifact-name`、`channel`、`skip-clean`、`build-target`、`hook-pre`、`hook-post` 和 `build-args`（JSON object 字符串，承载其他所有构建参数，如 `{"flavor":"dev","dart-define":{"APP_ENV":"dev"}}`）。
