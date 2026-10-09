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

Fastforge 分两步构建每个 Linux 包。第一步按安装后的路径搭好**包根目录**，也就是所有要安装的内容：

- bundle 放在 `/opt/<binary>`（AppImage 为根目录本身），`/usr/bin/<binary>` 链接指向其中的可执行文件；
- 桌面文件 `${APP_ID}.desktop`；
- 以 `${APP_ID}` 命名、放进 hicolor 主题的图标；
- AppStream metainfo 文件 `${APP_ID}.metainfo.xml`；
- `files/` 覆盖树。

第二步补上该格式的元数据：`DEBIAN/control`、RPM spec、`.PKGINFO` 或 `AppRun`。两步都可以用 `.fastforge/packaging/linux/` 中各格式自己的原始文件定制，这些文件会用[打包变量](../packaging.md#打包变量)渲染。每个原始文件只替换 Fastforge 原本会生成的那一个文件，因此可以先只提供一个文件，其余继续使用默认值。

| 目录        | 原始文件                                                                                                    |
| ----------- | ----------------------------------------------------------------------------------------------------------- |
| `shared/`   | 一个 `*.desktop` 文件、一个 `*.metainfo.xml` 文件和 `files/` 覆盖树，所有格式共用                          |
| `deb/`      | `control`、`preinst`、`postinst`、`prerm`、`postrm`、`config`、`conffiles`、`triggers`、`templates`、`shlibs`、`symbols` |
| `rpm/`      | 一个 `*.spec` 文件，文件名不限                                                                              |
| `pacman/`   | `PKGINFO`、`INSTALL`（也可以使用 `.PKGINFO`、`.INSTALL`）                                                    |
| `appimage/` | `AppRun`                                                                                                    |

每个格式目录也可以有自己的 `*.desktop` 文件、`*.metainfo.xml` 文件和 `files/` 覆盖树，优先于共用的版本。

```text
.fastforge/packaging/linux/
├── shared/
│   ├── ${APP_ID}.desktop
│   └── ${APP_ID}.metainfo.xml
├── deb/
│   ├── control
│   └── files/usr/share/doc/${PACKAGE_NAME}/copyright
├── rpm/${PACKAGE_NAME}.spec
├── pacman/PKGINFO
└── appimage/AppRun
```

[`examples/hello_world`](../../../examples/hello_world/.fastforge/packaging/linux/) 使用这种布局，不需要任何 `make_config.yaml`。

各格式共用的项目元数据写在 `.fastforge/config.yaml` 的 `project:` 中，详见[打包变量](../packaging.md#打包变量)：

```yaml
project:
  display_name: Hello World
  description: A Flutter counter demo packaged with fastforge
  homepage: https://example.com
  app_id: com.example.hello_world
  package_name: hello-world
  icon: assets/logo.png
  license: MIT
  maintainer: Jane Doe <jane@example.com>
```

- **渲染规则。** 只有 `NAME` 是打包变量时，`${NAME}` 才会被替换，其余内容保持原样。因此 `$1`、`${HOME}` 等 shell 展开、`%{buildroot}` 等 RPM 宏以及 `%U` 等桌面文件字段代码都无需转义。需要字面量 `${` 时写 `$${`。覆盖树中的文件名和目录名同样会被渲染，例如 `files/usr/share/doc/${PACKAGE_NAME}/copyright`。
- **覆盖树。** 先把共用的 `files/` 复制到包根目录，再复制格式目录中的，同名文件以后者为准。AppImage 不会安装 `/etc` 或 `/opt` 下的文件，因此只接收共用覆盖树中的 `usr/`。文本文件会被渲染，二进制文件原样复制，权限和符号链接保持不变。
- **自动补全。** DEB：control 中没有 `Installed-Size` 时自动补上，生成 `md5sums`，并把所有 `/etc` 文件登记到 `conffiles`。Pacman：补上 `size`、`builddate`，并为每个 `/etc` 文件添加 `backup` 条目。RPM：把 `/etc` 文件列为 `%config(noreplace)`。你自己写的值会保留。
- **桌面文件和 metainfo。** 模板的文件名不限：一个 `*.desktop` 文件和一个 `*.metainfo.xml`（或 `*.appdata.xml`）文件。无论叫什么，都会安装为 `${APP_ID}.desktop` 和 `usr/share/metainfo/${APP_ID}.metainfo.xml`，这是桌面文件规范和 AppStream 的要求。桌面文件中启动命令写作 `${APP_BINARY_NAME}`，图标写作 `${APP_ID}`。在 Linux 上 `APP_ID` 总有值：优先取 `project.app_id`，其次取 `linux/CMakeLists.txt` 中的 `APPLICATION_ID`，最后为可执行文件名。
- **Wayland。** 桌面文件按窗口的 app id 匹配，而 GTK 把程序名作为 app id 上报。请让 `linux/CMakeLists.txt` 中的 `APPLICATION_ID` 与 `project.app_id` 一致，或不设置 `project.app_id`。还要像新版 Flutter 模板那样在 `my_application_new()` 中调用 `g_set_prgname(APPLICATION_ID)`，否则窗口上报的是可执行文件名。
- **图标。** 图标取自 `project.icon`；`make_config.yaml` 设置了 `icon` 时以它为准。SVG 安装为 scalable 图标。PNG 会先补成正方形，再缩放到不超过原图的各个标准尺寸（16 到 512 像素）。其他格式会导致打包失败。如果想提供手工绘制的各尺寸图标，请放进 `files/` 覆盖树。
- **包名和版本号。** `PACKAGE_NAME` 优先取 `make_config.yaml` 的 `package_name`，其次取 `project.package_name`，最后为该格式的默认值。`PACKAGE_VERSION` 是按该格式语法书写的版本号，`1.2.3-beta.1` 这类预发布版本会排在正式版之前。
- 同一目录中有**多个同类模板**（例如两个 `*.desktop` 文件或两个 `*.spec` 文件）时打包失败，因为 Fastforge 无法判断应使用哪一个。

同时使用 Dart CLI 的项目仍可保留 `linux/packaging/<format>/make_config.yaml`，Fastforge 会用它生成没有原始文件的那些文件；新项目不需要它。常用键包括 `display_name`、`package_name`、`icon`、`metainfo`、`categories`、`keywords`、`generic_name` 和 `startup_notify`。文件不存在时使用默认值；文件无法解析时打包失败。既没有原始文件也没有 `make_config.yaml` 时，生成的元数据取自 `project:`，未设置许可证时写为 `LicenseRef-Unknown`。

## AppImage

需要 `appimagetool` 和 `ldd`。AppImage 只能在 glibc 不旧于构建主机的系统上运行，请在所支持的最老发行版上构建。默认 `AppRun` 会优先使用随包的库，并在当前目录下带着传入的参数启动程序。插件依赖的共享库会复制到 `usr/lib`，`files/` 覆盖树已经提供的除外。`make_config.yaml` 额外的键包括 `include`（额外的共享库，通过 `locate` 查找，此时需要安装 `locate`）和 `actions`。

## DEB

需要 `dpkg-deb`（Debian/Ubuntu 上由 `dpkg-dev` 提供）。文件属主为 root。默认包名为可执行文件名转为小写，并把 Debian 不允许的字符（如 `_`）替换为 `-`。`Version` 保留构建号（`1.2.3+4`），预发布版本中的 `-` 写成 `~`。`make_config.yaml` 未设置 `dependencies` 时，包依赖 `libgtk-3-0`；设为空列表可以去掉。`/usr/bin` 链接已包含在包里，因此只有提供了维护脚本，或设置了 `postinstall_scripts`/`postuninstall_scripts` 时才会写入维护脚本。`make_config.yaml` 额外的键包括 `maintainer`、`co_authors`、`priority`、`section` 和 `dependencies`。

## RPM

需要 `rpmbuild` 和 `patchelf`。包根目录位于 `${PACKAGING_DIRECTORY}`，它的 `%files` 清单位于 `${RPM_FILE_LIST}`，因此 spec 只需两行即可安装。无论 spec 自己叫什么，都会写成 `SPECS/${PACKAGE_NAME}.spec`。

```spec
%global __provides_exclude_from ^${INSTALL_DIR}/.*$
%global __requires_exclude ^(${RPM_PRIVATE_LIBS})

%install
cp -a ${PACKAGING_DIRECTORY}/. %{buildroot}/

%files -f ${RPM_FILE_LIST}
```

清单包含文件、链接以及包自己创建的所有目录（如 `/opt/<binary>`、`/etc/<app id>`），但不占用 `/usr/share/applications` 等系统目录。两行 `%global` 把随包的 Flutter 引擎和插件排除在自动生成的 `Provides` 和 `Requires` 之外，GTK、glibc 等系统库依赖仍会保留。`RPM_PRIVATE_LIBS` 列出 bundle 中的库，并已按 spec 的需要转义。生成的 spec 也会这样处理，并关闭 debuginfo 包和 `.build-id` 链接，否则带有相同引擎的多个应用会冲突。`Version` 为构建名，其中的 `-` 转为 `~`；`Release` 为构建号。这些变量同时设置在 rpmbuild 的环境中，因此也可以使用 `%{getenv:APP_VERSION}`。产物会从任意 `RPMS/<arch>/` 目录中查找，spec 可以自行设置 `BuildArch`。`make_config.yaml` 额外的键包括 `summary`、`group`、`vendor`、`packager`、`license`、`url`、`requires` 和 `build_arch`。

## Pacman

需要支持 zstd 的 `bsdtar`，不使用 `makepkg`。产物是属主为 root 的 `.pkg.tar.zst` 归档，包含 `.MTREE`、`.PKGINFO`、`.INSTALL`（如有）以及包根目录。生成的 `.PKGINFO` 使用 makepkg 的格式，每个值一行 `key = value`。`pkgver` 为 `<构建名>-<构建号>`，构建名中的 `-` 会被去掉。`make_config.yaml` 未设置 `dependencies` 时，包依赖 `gtk3`；设为空列表可以去掉。`make_config.yaml` 额外的键包括 `maintainer`、`licenses`、`groups`、`dependencies`、`optional_dependencies` 和 `postinstall_scripts`。

## ZIP 和 direct

`zip` 把 bundle 目录压缩为归档；`direct` 把它直接复制到 `dist/<version>/`。两者都不需要额外工具。

返回[打包器总览](README.md)。
