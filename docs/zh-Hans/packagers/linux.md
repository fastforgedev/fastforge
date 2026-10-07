# Linux

[English](../../en/packagers/linux.md) | 简体中文

Fastforge 支持把 Flutter Linux 应用打包为 [AppImage](#appimage)、[DEB](#deb)、[RPM](#rpm)、[Pacman](#pacman)、[ZIP 或直接复制](#zip-和-direct)。

## 当前状态

| 构建系统        | `package` 状态                                          |
| --------------- | ------------------------------------------------------- |
| Flutter Builder | AppImage、DEB、RPM、Pacman、ZIP、direct 已通过 CLI/action 接入 |

Linux 打包需要 Flutter 项目（含 `pubspec.yaml`）。构建只能在 Linux 宿主上执行；在其他宿主上，`fastforge package` 会输出警告并跳过该 target。打包器使用 bundle 目录 `build/linux/<x64|arm64>/<mode>/bundle/`，产物写入 `dist/<version>/`。

```bash
fastforge package --targets deb,appimage
```

可执行文件名读取自 `linux/CMakeLists.txt` 中的 `set(BINARY_NAME "...")`，读取不到时使用 `pubspec.yaml` 中的名称。

## 配置

AppImage、DEB、RPM 和 Pacman 使用各格式自己的原始文件配置，放在 `linux/packaging/<format>/` 下。Fastforge 用[打包变量](../packaging.md#打包变量)渲染这些文件后原样放入包中。每个原始文件只替换 Fastforge 原本会生成的那一个文件，因此可以先只提供一个文件，其余继续使用默认值。

| 格式     | 原始文件                                                                                                             | 放置位置                    |
| -------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| DEB      | `control`、`postinst`、`postrm`；以及 `preinst`、`prerm`、`config`、`conffiles`、`triggers`、`templates`、`shlibs`、`symbols` | `DEBIAN/`                    |
| RPM      | 一个 `*.spec` 文件                                                                                                    | `SPECS/`                     |
| Pacman   | `PKGINFO`、`INSTALL`（也可以使用带点的 `.PKGINFO`、`.INSTALL`）                                                        | `.PKGINFO`、`.INSTALL`       |
| AppImage | `AppRun`                                                                                                              | AppDir 根目录                |
| 四种格式 | 一个 `*.desktop` 文件                                                                                                 | 默认生成的桌面文件所在位置  |
| 四种格式 | `files/` 目录                                                                                                         | 复制到包根目录（见下文）    |

```text
linux/packaging/deb/
├── control
├── postinst
├── hello.desktop
└── files/
    └── etc/${PACKAGE_NAME}/app.conf
```

```text
Package: ${PACKAGE_NAME}
Version: ${APP_VERSION}
Architecture: ${PACKAGE_ARCH}
Maintainer: Example Team <${SUPPORT_EMAIL}>
Depends: libgtk-3-0
Description: ${APP_DESCRIPTION}
```

- **渲染规则。** 只有 `NAME` 是打包变量时，`${NAME}` 才会被替换，其余内容保持原样。因此 `$1`、`${HOME}` 等 shell 展开、`%{buildroot}` 等 RPM 宏以及 `%U` 等桌面文件字段代码都无需转义。需要字面量 `${` 时写 `$${`。
- **`files/` 覆盖树**会复制到 DEB 和 Pacman 的包根目录、AppDir 或 RPM 的构建目录。文件名和目录名同样会被渲染。文本文件会被渲染，二进制文件原样复制，权限和符号链接保持不变。
- **替换文件也会替换默认行为。** 默认生成的 `postinst` 和 Pacman `INSTALL` 会创建 `/usr/bin/<binary>` 符号链接。改用原始文件后，如仍需要该链接，需要自己写，例如 `ln -sf ${INSTALL_DIR}/${APP_BINARY_NAME} /usr/bin/${APP_BINARY_NAME}`。
- **RPM。** bundle 放在 `${PACKAGING_DIRECTORY}/${APP_NAME}/`，`files/` 覆盖树放在 `${PACKAGING_DIRECTORY}/`，原始 spec 在 `%install` 中从这些位置复制。这些变量同时设置在 rpmbuild 的环境中，因此也可以使用 `%{getenv:APP_VERSION}`。产物会从任意 `RPMS/<arch>/` 目录中查找，spec 可以自行设置 `BuildArch`。
- **图标。** 图标取自 `make_config.yaml` 的 `icon`，没有时取 `.fastforge/config.yaml` 的 `project.icon`。DEB 和 Pacman 安装到 `usr/share/icons/hicolor/{128x128,256x256}/apps/<binary>`，RPM 放到构建目录中的 `<binary>.<ext>`，AppImage 放到 AppDir 根目录的 `<应用名>.<ext>`，这是 `appimagetool` 的要求。
- 同一格式目录中有**多个 `*.desktop` 或 `*.spec` 文件**时打包失败，因为 Fastforge 无法判断应使用哪一个。

为保持兼容，仍会读取结构与 Dart CLI 一致的 `linux/packaging/<format>/make_config.yaml`，用它生成没有原始文件的那些文件。常用键包括 `display_name`、`package_name`、`icon`、`metainfo`、`categories`、`keywords`、`generic_name` 和 `startup_notify`。文件不存在时使用默认值；文件无法解析时打包失败。DEB 的默认包名为可执行文件名转为小写，并把 Debian 不允许的字符（如 `_`）替换为 `-`。

## AppImage

需要 `appimagetool` 和 `ldd`。`make_config.yaml` 额外的键包括 `include`（额外的共享库，通过 `locate` 查找，此时需要安装 `locate`）和 `actions`。

## DEB

需要 `dpkg-deb`（Debian/Ubuntu 上由 `dpkg-dev` 提供）。`make_config.yaml` 额外的键包括 `maintainer`、`co_authors`、`priority`、`section`、`dependencies` 和 `postinstall_scripts`。

## RPM

需要 `rpmbuild` 和 `patchelf`。默认生成的 spec 兼容 rpm 4.20 及以后版本，这些版本在独立的构建子目录中执行 `%install`。`make_config.yaml` 额外的键包括 `summary`、`group`、`vendor`、`packager`、`license`、`url`、`requires` 和 `build_arch`。

## Pacman

需要 `bsdtar` 和 `xz`，不使用 `makepkg`。归档包含 `.PKGINFO`、`.INSTALL` 以及包根目录下的所有顶层目录，包括来自 `files/` 的目录。`make_config.yaml` 额外的键包括 `maintainer`、`licenses`、`dependencies`、`optional_dependencies` 和 `postinstall_scripts`。

## ZIP 和 direct

`zip` 把 bundle 目录压缩为归档；`direct` 把它直接复制到 `dist/<version>/`。两者都不需要额外工具。

返回[打包器总览](README.md)。
