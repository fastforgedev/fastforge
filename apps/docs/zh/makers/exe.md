# exe

使用 Inno Setup 将 Flutter 应用构建为 Windows EXE 安装程序。这将创建一个传统的 Windows 安装可执行文件，引导用户完成安装过程，包括创建桌面快捷方式和开始菜单条目。

## 环境要求

- Windows 系统
- [`Inno Setup 6`](https://jrsoftware.org/isinfo.php) — 一个免费的 Windows 程序安装程序制作工具

## 使用方法

将 `make_config.yaml` 添加到你的项目 `windows/packaging/exe` 目录。

```yaml
# AppId 的值唯一标识此应用。
# 不要在其他应用的安装程序中使用相同的 AppId 值。
app_id: 5B599538-42B1-4826-A479-AF079F21A65D
publisher: LeanFlutter
publisher_url: https://github.com/fastforgedev/fastforge
display_name: Hello 世界
create_desktop_icon: true
# 参见：https://jrsoftware.org/ishelp/index.php?topic=setup_defaultdirname
# install_dir_name: "D:\\HELLO-WORLD"
# 这里的路径是相对于项目根目录的路径；图标格式必须是 ico 格式，不能是 png 或其他格式
# setup_icon_file: windows\runner\resources\app_icon.ico
locales:
  - en
  - zh
# 允许安装程序运行的 CPU 架构标识符列表（空格分隔）。
# 参见：https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesallowed
# 默认值为 `x64compatible`。
# architectures_allowed: x64compatible

# 触发 64 位安装模式的架构标识符列表（空格分隔）。
# 参见：https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesinstallin64bitmode
# 默认值为 `x64compatible`。
# architectures_install_in_64bit_mode: x64compatible
```

运行：

```
fastforge package --platform windows --targets exe
```

## 高级用法

### 自定义 Inno Setup 安装路径

默认情况下，`fastforge` 会在默认安装路径（`C:\Program Files (x86)\Inno Setup 6`）下查找 Inno Setup。如果你将 Inno Setup 安装在自定义位置（例如通过 [Scoop](https://scoop.sh) 安装或使用便携版），可以通过 `INNO_SETUP_PATH` 环境变量来指定路径。

```bash
# PowerShell
$env:INNO_SETUP_PATH = "D:\Tools\Inno Setup 6"
fastforge package --platform windows --targets exe

# CMD
set INNO_SETUP_PATH=D:\Tools\Inno Setup 6
fastforge package --platform windows --targets exe
```

如果没有设置 `INNO_SETUP_PATH`，`fastforge` 会先检查默认路径，然后回退到在系统 `PATH` 中查找 `iscc`（适用于通过 Scoop 安装或手动将 Inno Setup 添加到 PATH 的场景）。

### 目标架构

默认情况下，生成的安装程序允许在 **x64 兼容的系统**上安装（包括原生 x64 和带 x64 模拟的 ARM64 系统）。你可以通过 `architectures_allowed` 和 `architectures_install_in_64bit_mode` 选项自定义此行为。

```yaml
# 允许在 x64 和 ARM64 系统上安装
architectures_allowed: x64compatible
architectures_install_in_64bit_mode: x64compatible
```

- `architectures_allowed` — 指定允许安装程序运行的 CPU 架构。值为空格分隔的架构标识符或布尔表达式。参见 [Inno Setup 文档](https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesallowed) 了解可用值。
- `architectures_install_in_64bit_mode` — 指定应触发 64 位安装模式的架构（例如默认安装目录 `{autopf64}`）。值为空格分隔的架构标识符或布尔表达式。参见 [Inno Setup 文档](https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesinstallin64bitmode) 了解可用值。

常用架构标识符：`x86`、`x64`、`arm64`、`x64compatible`、`x86compatible`。其中 `x64compatible` 会匹配原生 x64 系统和运行 x64 模拟的 ARM64 系统（如 Windows 11 on ARM），而 `x64` 仅匹配原生 x64 硬件。

如果未指定，这两个选项默认值均为 `x64compatible`。

### 自定义 Inno Setup 模板

默认情况下，`fastforge` 会在构建时基于内部模板生成一个 Inno Setup 配置（`.iss`），并填充 `make_config.yaml` 中提供的值。如果你需要对 Inno Setup 配置进行更多控制，可以使用 `script_template` 选项提供自定义模板。

例如：

1. 添加 `script_template: inno_setup.iss` 到你的 `make_config.yaml`
2. 在同一目录中创建 `inno_setup.iss`
3. 从源代码中复制 [原始模板](https://github.com/fastforgedev/fastforge/blob/main/packages/flutter_app_packager/lib/src/makers/exe/inno_setup/inno_setup_script.dart) 并进行调整。

## 相关链接

- [Inno Setup 官网](https://jrsoftware.org/isinfo.php)
- [Inno Setup 文档](https://jrsoftware.org/ishelp/)
