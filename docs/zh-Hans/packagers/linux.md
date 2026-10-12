# Linux

[English](../../en/packagers/linux.md) | 简体中文

Fastforge 支持把 Flutter Linux 应用打包为 [AppImage](#appimage)、[DEB](#deb)、[RPM](#rpm)、[Pacman](#pacman)、[Flatpak](#flatpak)、[ZIP 或直接复制](#zip-和-direct)。

## 当前状态

| 构建系统        | `package` 状态                                          |
| --------------- | ------------------------------------------------------- |
| Flutter Builder | AppImage、DEB、RPM、Pacman、Flatpak、ZIP、direct 已通过 CLI/action 接入 |

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
- **窗口位置。** Wayland 下应用无法自行放置窗口，因此从桌面启动安装包时，`window_manager` 的 `setPosition` 等调用不会生效，而同一 bundle 在 X11 终端或 IDE 中启动时可以定位窗口。安装包内容就是 bundle 本身，区别只在会话类型。必须定位窗口的应用可以在桌面模板中写 `Exec=env GDK_BACKEND=x11 ${APP_BINARY_NAME} %U`，改为通过 XWayland 运行。
- **图标。** 图标取自 `project.icon`；`make_config.yaml` 设置了 `icon` 时以它为准。SVG 安装为 scalable 图标。PNG 会先补成正方形，再缩放到不超过原图的各个标准尺寸（16 到 512 像素）。其他格式会导致打包失败。如果想提供手工绘制的各尺寸图标，请放进 `files/` 覆盖树。
- **包名和版本号。** `PACKAGE_NAME` 优先取 `make_config.yaml` 的 `package_name`，其次取 `project.package_name`，最后为该格式的默认值。`PACKAGE_VERSION` 是按该格式语法书写的版本号，`1.2.3-beta.1` 这类预发布版本会排在正式版之前。
- 同一目录中有**多个同类模板**（例如两个 `*.desktop` 文件或两个 `*.spec` 文件）时打包失败，因为 Fastforge 无法判断应使用哪一个。

同时使用 Dart CLI 的项目仍可保留 `linux/packaging/<format>/make_config.yaml`，Fastforge 会用它生成没有原始文件的那些文件；新项目不需要它。常用键包括 `display_name`、`package_name`、`icon`、`metainfo`、`categories`、`keywords`、`generic_name` 和 `startup_notify`。文件不存在时使用默认值；文件无法解析时打包失败。既没有原始文件也没有 `make_config.yaml` 时，生成的元数据取自 `project:`，未设置许可证时写为 `LicenseRef-Unknown`。

## AppImage

需要 `appimagetool` 和 `ldd`。AppImage 只能在 glibc 不旧于构建主机的系统上运行，请在所支持的最老发行版上构建。默认 `AppRun` 会优先使用随包的库，并在当前目录下带着传入的参数启动程序。插件依赖的共享库会复制到 `usr/lib`，同名文件只复制一份，`files/` 覆盖树已经提供的除外；`ldd` 无法分析的库会给出警告并跳过。`make_config.yaml` 额外的键包括 `include`（额外的共享库：绝对路径按通配符展开并打包全部匹配项，如 `/usr/lib/x86_64-linux-gnu/libgphoto2/*/*.so`；其他条目通过 `locate` 查找，此时需要安装 `locate`，普通名称取第一个结果，通配符模式取全部结果）、`actions`（由 `label`、`name` 和 `arguments` 组成的条目，写成 `[Desktop Action]` 段）以及 `supported_mime_type`。AppImage 的 `make_config.yaml` 没有 `package_name`。

如需嵌入[更新信息](https://github.com/AppImage/AppImageSpec/blob/master/draft.md#update-information)，在 `.fastforge/config.yaml` 的 `env:` 或环境变量中设置 `UPDATE_INFORMATION`。安装了 `zsyncmake` 时，`appimagetool` 还会在 AppImage 旁生成 `.zsync` 文件。如需使用其他 AppImage runtime，用同样的方式设置 `APPIMAGETOOL_RUNTIME_FILE`。

GTK、X11 和 Wayland 库（包括光标相关库）不会打包进来，默认 `AppRun` 也只修改 `LD_LIBRARY_PATH`，因此应用使用主机的 GTK 和光标主题。`Unable to load ... from the cursor theme` 之类的警告来自主机主题，其他 GTK 应用也会出现：请确认主题已安装（例如 `/usr/share/icons/Adwaita/cursors`），或设置 `XCURSOR_THEME`。

```yaml
env:
  UPDATE_INFORMATION: gh-releases-zsync|owner|repo|latest|*x86_64.AppImage.zsync
```

## DEB

需要 `dpkg-deb`（Debian/Ubuntu 上由 `dpkg-dev` 提供）。文件属主为 root。包使用 xz 而非 zstd 压缩，因此 Debian 11、deepin、统信 UOS 等较旧的 dpkg 也能安装。默认包名为可执行文件名转为小写，并把 Debian 不允许的字符（如 `_`）替换为 `-`。`Version` 保留构建号（`1.2.3+4`），预发布版本中的 `-` 写成 `~`。`make_config.yaml` 未设置 `dependencies` 时，包依赖 `libgtk-3-0`；设为空列表可以去掉。`/usr/bin` 链接由包管理器拥有并负责卸载清理。旧 Dart 包的 `postrm` 可能在升级时删除它，因此默认 `postinst` 会在配置阶段、执行 `postinstall_scripts` 之前，仅在入口缺失时补回链接。已有文件及符号链接（包括失效链接）均保留，覆盖树提供的链接目标也会保留。原生 `postinst` 会替换默认脚本，需要自行处理这项升级兼容；仅配置了 `postuninstall_scripts` 时才生成 `postrm`。维护脚本以 root 身份运行，包管理器的动作作为 `$1` 传入（`postrm` 中如 `remove`、`purge` 或 `upgrade`），因此 `~` 指向 root 的主目录而不是用户的：脚本无法清理 `~/.local/share/<app>` 等用户数据，Debian 规范也将其留给用户处理。只应删除系统级文件，并通过 `$1` 跳过升级，例如 `[ "$1" = purge ] && rm -rf /var/lib/<app>`。`make_config.yaml` 额外的键包括 `maintainer`、`co_authors`、`priority`、`section`、`essential`、`installed_size`、`postinstall_scripts`、`postuninstall_scripts`、`supported_mime_type`、`actions`、`startup_wm_class`，以及依赖关系列表 `dependencies`、`pre_dependencies`、`recommended_dependencies`、`suggested_dependencies`、`enhances`、`breaks`、`conflicts`、`provides`、`replaces`、`build_dependencies` 和 `build_dependencies_indep`。

## RPM

需要 `rpmbuild` 和 `patchelf`。包根目录位于 `${PACKAGING_DIRECTORY}`，它的 `%files` 清单位于 `${RPM_FILE_LIST}`，因此 spec 只需两行即可安装。无论 spec 自己叫什么，都会写成 `SPECS/${PACKAGE_NAME}.spec`。

```spec
%global __provides_exclude_from ^${INSTALL_DIR}/.*$
%global __requires_exclude ^(${RPM_PRIVATE_LIBS})

%install
cp -a ${PACKAGING_DIRECTORY}/. %{buildroot}/

%files -f ${RPM_FILE_LIST}
```

清单包含文件、链接以及包自己创建的所有目录（如 `/opt/<binary>`、`/etc/<app id>`），但不占用 `/usr/share/applications` 等系统目录。两行 `%global` 把随包的 Flutter 引擎和插件排除在自动生成的 `Provides` 和 `Requires` 之外，GTK、glibc 等系统库依赖仍会保留。`RPM_PRIVATE_LIBS` 列出 bundle 中的库，并已按 spec 的需要转义。生成的 spec 也会这样处理，并关闭 debuginfo 包和 `.build-id` 链接，否则带有相同引擎的多个应用会冲突。默认包名为 `pubspec.yaml` 中的名称。`Version` 为构建名，其中的 `-` 转为 `~`；`Release` 为构建号按 `.` 分隔的第一段（没有构建号时为 `1`），后接 `%{?dist}`。这些变量同时设置在 rpmbuild 的环境中，因此也可以使用 `%{getenv:APP_VERSION}`。产物会从任意 `RPMS/<arch>/` 目录中查找，spec 可以自行设置 `BuildArch`。`make_config.yaml` 额外的键包括 `summary`、`description`、`group`、`vendor`、`packager`、`packager_email`、`license`、`url`、`requires`、`build_requires`、`build_arch`、`postinstall_scripts`（`%post`）、`postun` 和 `postuninstall_scripts`（`%postun`）、`spec_macros`（追加在生成的 `%global` 宏之后的行）、`supported_mime_type` 以及 `actions`。

RPM 保留 `/opt/<binary>` 安装目录。包名与程序名不同的项目同时提供 `/usr/bin/<binary>` 和旧版 `/usr/bin/<package>` 启动命令，均指向原生安装目录。迁移时保持包名不变，RPM 升级才能清理旧包拥有的 `/usr/share/<package>` 文件；额外的非托管文件或自定义卸载脚本需要项目自行迁移。

## Pacman

需要支持 zstd 的 `bsdtar`（Debian/Ubuntu 上为 `libarchive-tools`，Fedora 上为 `bsdtar`，Arch 上为 `libarchive`），不使用 `makepkg`。产物是属主为 root 的 `.pkg.tar.zst` 归档，包含 `.MTREE`、`.PKGINFO`、`.INSTALL`（如有）以及包根目录。生成的 `.PKGINFO` 使用 makepkg 的格式，每个值一行 `key = value`。默认包名为可执行文件名。`pkgver` 为 `<构建名>-<release>`，构建名中的 `-` 会被去掉；release 为构建号按 `.` 分隔的第一段，没有构建号时为 `1`。设置了 `SOURCE_DATE_EPOCH` 时，`builddate` 使用该值。`make_config.yaml` 未设置 `dependencies` 时，包依赖 `gtk3`；设为空列表可以去掉。`make_config.yaml` 额外的键包括 `maintainer`、`licenses`、`groups`、`dependencies`、`optional_dependencies`、`conflicts`、`provides`、`replaces`、`supported_mime_type`、`actions`，以及用于生成 `.INSTALL` 的 `postinstall_scripts`、`postupgrade_scripts` 和 `postuninstall_scripts`。

Pacman 旧字段 `installed_size` 和 `options` 现在会输出迁移提示。`size` 按实际暂存内容计算字节数；`options` 属于 makepkg，不能作为 PKGINFO 元数据。需要的预处理应移到钩子或工作流。

## Flatpak

需要 `flatpak` 和 `flatpak-builder`。产物是单文件 bundle（`.flatpak`），用户通过 `flatpak install --user <file>.flatpak` 安装；其中注明 Flathub 为运行时所在仓库，安装时会一并安装运行时。

应用暂存为 `/app` 的内容：bundle 位于 `/app/lib/<binary>`，另有 `/app/bin/<binary>`，以及以 app ID 命名、由 Flatpak 导出到主机的桌面入口、图标和 metainfo。共享 `files/` 覆盖树只有 `usr/` 部分生效，对应 `/app`。`APP_ID` 必须是至少三段的合法 Flatpak ID，例如 `com.example.App`；请设置 `project.app_id`，并让 `linux/CMakeLists.txt` 中的 `APPLICATION_ID` 与之一致。

生成的清单使用 GNOME 运行时（`org.gnome.Platform` 48，提供 GTK 3），允许访问 Wayland、作为后备的 X11、GPU、IPC 和网络。缺少 Flathub 时 Fastforge 会将其添加为用户级远程仓库，由 `flatpak-builder` 为当前用户安装运行时和 SDK。可在 `.fastforge/config.yaml` 的 `env:` 或环境变量中设置 `FLATPAK_RUNTIME`、`FLATPAK_RUNTIME_VERSION`、`FLATPAK_SDK` 进行更改。需要其他权限或模块时，在 `.fastforge/packaging/linux/flatpak/` 中放置清单（`*.yml`、`*.yaml` 或 `*.json`），它会替换生成的清单，并可从 `${PACKAGING_DIRECTORY}` 复制暂存目录：

```yaml
id: ${APP_ID}
runtime: org.gnome.Platform
runtime-version: '48'
sdk: org.gnome.Sdk
command: ${APP_BINARY_NAME}
finish-args: [--share=ipc, --socket=fallback-x11, --socket=wayland, --device=dri, --filesystem=xdg-download]
modules:
  - name: ${APP_BINARY_NAME}
    buildsystem: simple
    build-commands: [cp -a . /app/]
    sources:
      - type: dir
        path: ${PACKAGING_DIRECTORY}
```

bundle 由预先构建的 Flutter 输出生成，适合直接分发文件；上架 Flathub 本身要求从源码构建。

## Studio 表单

在本地 Studio 中打开项目，进入「设置 → 构建与打包」，即可编辑
`.fastforge/packaging/linux/deb/control`、Pacman 的 `PKGINFO`（或已有的
`.PKGINFO`），以及共享目录中唯一的 `*.desktop` 文件。表单支持配置元数据、
依赖和启动器字段，预览原生文件，并从模板创建缺失文件。保存时保留变量、注释、
未覆盖的字段和桌面启动器的其他分组；如果加载后文件被外部修改，必须重新加载后
才能保存。这些表单用于 Rust 原生打包器，不修改 Dart 的旧版配置文件。

## ZIP 和 direct

`zip` 把 bundle 目录压缩为归档；`direct` 把它直接复制到 `dist/<version>/` 下以产物名（不含扩展名）命名的目录。两者都不需要额外工具。

返回[打包器总览](README.md)。
