# Windows

[English](../../en/packagers/windows.md) | 简体中文

Fastforge 支持把 Flutter Windows 应用打包为 [EXE](#exe)、[MSIX](#msix)、[ZIP 或直接复制](#zip-和-direct)。

## 当前状态

| 构建系统        | `package` 状态                           |
| --------------- | ---------------------------------------- |
| Flutter Builder | EXE、MSIX、ZIP、direct 已通过 CLI/action 接入 |

Windows 打包需要 Flutter 项目（含 `pubspec.yaml`）。构建只能在 Windows 宿主上执行；在其他宿主上，`fastforge package` 会输出警告并跳过该 target。打包器使用 runner 目录 `build/windows/<arch>/runner/<Mode>/`（Flutter 3.15 以前为 `build/windows/runner/<Mode>/`），产物写入 `dist/<version>/`。

```bash
fastforge package --targets exe,zip
```

## 配置

Windows 与 Linux 采用相同的两步模型：先准备应用安装目录，再加入格式专属的原生元数据。通过 `.fastforge/packaging/windows/` 下的原生文件定制，并使用[打包变量](../packaging.md#打包变量)渲染：

```text
.fastforge/packaging/windows/
├── shared/files/                 # EXE 与 MSIX 共用的应用文件
├── exe/
│   ├── setup.iss                 # 一个 *.iss，名称不限
│   └── files/                    # EXE 专属应用文件
└── msix/
    ├── AppxManifest.xml          # 替换默认生成的清单
    └── files/Assets/             # MSIX 图标或其他应用文件
```

依次复制构建输出、`shared/files/` 和格式自身的 `files/`，后者覆盖前者。文件名、目录名和文本内容会渲染变量，二进制文件保持原样。原生元数据替换默认生成的脚本或清单。EXE 目录中存在多个 `*.iss` 会报错。ZIP 和 direct 继续直接处理构建输出。

公共元数据来自 `.fastforge/config.yaml` 的 `project:`，包括 `display_name`、`description`、`homepage`、`app_id` 和 `icon`。Windows 图标支持 PNG 或 ICO：EXE 的默认脚本把项目 PNG 转为 ICO，MSIX 生成对应尺寸的 PNG。`files/Assets/` 中提供的 MSIX 图标保持原样。

Rust CLI 仍读取 `windows/packaging/<format>/make_config.yaml`；MSIX 还合并 `pubspec.yaml` 的 `msix_config`，以前者为准。旧字段覆盖 `project:` 提供的生成默认值；原生文件优先于生成元数据和旧 EXE `script_template`。新项目只需 `project:` 和原生文件，参见 [`examples/hello_world`](../../../examples/hello_world/.fastforge/packaging/windows/)。

## EXE

EXE 使用 Inno Setup 生成安装程序。它是唯一被标记为安装包的格式，因此默认产物名以 `-setup.exe` 结尾。

需要 Inno Setup 6。Fastforge 依次从 `INNO_SETUP_PATH`、`C:\Program Files (x86)\Inno Setup 6`、`PATH` 中的 `iscc` 查找编译器。`INNO_SETUP_PATH` 也可放在 `.fastforge/config.yaml` 的 `env:` 中。编译时显式指定输出目录和文件名，因此原生脚本也使用配置的产物名。

最小 `exe/setup.iss` 示例：

```ini
[Setup]
AppId=${APP_ID}
AppName=${APP_DISPLAY_NAME}
AppVersion=${BUILD_NAME}
DefaultDirName=${INSTALL_DIR}
OutputBaseFilename=${OUTPUT_BASE_FILENAME}

[Files]
Source: "${PACKAGING_DIRECTORY}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\${APP_DISPLAY_NAME}"; Filename: "{app}\${EXECUTABLE_NAME}"
```

`{app}` 等 Inno 常量和预处理语法保持原样。渲染后的脚本位于暂存目录旁，应用文件和编译资源请通过 `${PACKAGING_DIRECTORY}` 引用。`EXECUTABLE_NAME` 优先使用 `<APP_BINARY_NAME>.exe`，避免选到随包附带的辅助程序。没有原生脚本时，默认脚本使用项目元数据，并从构建路径判断 ARM64 或 x64。

旧 `make_config.yaml` 支持 `app_id`、`publisher_name`、`publisher_url`、`display_name`、`executable_name`、`install_dir_name`、`setup_icon_file`、`locales`、`create_desktop_icon`、`launch_at_startup`、`privileges_required` 和 `script_template` 等字段；旧自定义脚本继续使用 Liquid 语法。

## MSIX

需要 `PATH` 中的 Windows SDK `makeappx`，签名时还需要 `signtool`。Rust CLI 统一使用原生打包流程，项目是否依赖 Dart `msix` 包不再影响打包路径。

将原生清单放在 `msix/AppxManifest.xml`。这个文件中的变量值会自动进行 XML 转义，XML 标记保持原样。使用 `${APP_ID}` 作为应用标识、`${PACKAGE_VERSION}` 作为四段版本、`${PACKAGE_ARCH}` 作为架构、`${EXECUTABLE_NAME}` 作为可执行文件名、`${MSIX_PUBLISHER}` 作为发布者。标识需符合 MSIX 命名规则，例如不含下划线的 `dev.example.demo`。

未提供的元数据依次从旧配置、`project:` 和默认值生成。默认标识为 `com.flutter.<去掉下划线的应用名>`，版本为 `a.b.c.0`，不使用构建号。发布者默认为 `CN=Publisher`，应通过 `MSIX_PUBLISHER` 设置为与签名证书或商店身份一致的值。图标依次取 `logo_path`、`project.icon`、Flutter runner 图标或占位图；已提供的图标优先于自动生成。

```yaml
env:
  MSIX_PUBLISHER: CN=Example Company
  MSIX_CERTIFICATE_PATH: ${MSIX_CERTIFICATE_PATH}
  MSIX_CERTIFICATE_PASSWORD: ${MSIX_CERTIFICATE_PASSWORD}
```

这些变量也可直接通过进程环境传入。旧 `certificate_path`、`certificate_password` 和 `signtool_options` 继续支持。优先级为显式 packager 参数、环境变量、旧配置。未提供签名配置时生成未签名包并输出警告；旧 `store: true` 或 `sign_msix: false` 会跳过签名。

**兼容性变化：** Rust 不再运行 `dart run msix:create`，也不再使用其内置测试证书。`debug` 和 `install_certificate` 不会安装证书，`trim_logo` 不会应用；开发证书需单独安装。capability、文件关联、协议、执行别名和开机启动可直接写入清单；默认生成清单时仍支持对应旧字段。

## ZIP 和 direct

`zip` 把 runner 目录压缩为归档；`direct` 把它直接复制到 `dist/<version>/`。两者都不需要额外工具。

返回[打包器总览](README.md)。
