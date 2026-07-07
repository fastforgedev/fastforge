# rpm

将 Flutter 应用构建为 RPM 包，用于安装在基于 Red Hat 的 Linux 发行版上，如 Fedora、RHEL、CentOS 和 OpenSUSE。RPM 是这些发行版使用的标准包管理格式。

## 环境要求

- Linux 系统（推荐使用 Fedora/RHEL 系发行版）
- [patchelf](https://github.com/NixOS/patchelf) — 用于修改 ELF 二进制文件
- [rpmbuild](https://rpm-packaging-guide.github.io/#prerequisites) — RPM 包构建工具

安装要求：

- Debian/Ubuntu：`apt install rpm patchelf`
- Fedora：`dnf install gcc rpm-build rpm-devel rpmlint make python bash coreutils diffutils patch rpmdevtools patchelf`
- Arch：`yay -S rpmdevtools patchelf` 或 `pamac install rpmdevtools patchelf`

## 使用方法

将 `make_config.yaml` 添加到您的项目 `linux/packaging/rpm` 目录。

```yaml
icon: assets/logo.png
summary: A really cool application
group: Application/Emulator
vendor: Kingkor Roy Tirtho
packager: Kingkor Roy Tirtho
packager_email: krtirtho@gmail.com
license: GPLv3
url: https://github.com/fastforgedev/fastforge

display_name: Hello World

keywords:
  - Hello
  - World
  - Test
  - Application

generic_name: Cool Application

categories:
  - Cool
  - Awesome

startup_notify: true

# RPM spec 宏 — 在 .spec 文件顶部注入全局 RPM 指令
#
# 用于解决跨发行版 ABI 冲突、排除自动依赖检测或设置构建 ID 选项。
# 每个条目将按原样放置在生成的 .spec 文件最顶部，位于 preamble 之前。
#
# spec_macros:
#   - "%global __requires_exclude .*libcurl.*CURL_OPENSSL.*"
#   - "%define _build_id_links none"

# 您也可以指定 [metainfo](https://freedesktop.org/software/appstream/docs/chap-Quickstart.html) 文件
# 其中包含应用的元数据。
# metainfo: linux/packaging/myappid.appdata.xml
```

运行：

```
fastforge package --platform linux --targets rpm
```

## 安装结构

安装后，应用程序文件位于 `/opt/{package_name}/` 下。这遵循了[文件系统层次结构标准 (FHS)](https://refspecs.linuxfoundation.org/FHS_3.0/fhs/index.html) 中关于第三方附加软件包的规定：

| 路径 | 说明 |
|---|---|
| `/opt/{package_name}/` | 应用二进制文件和运行时文件 |
| `/usr/bin/{package_name}` | 主程序的符号链接（已添加到 `$PATH`） |
| `/usr/share/applications/{package_name}.desktop` | 应用程序菜单的桌面条目 |
| `/usr/share/icons/hicolor/128x128/apps/{package_name}.png` | 应用图标 |
| `/usr/share/icons/hicolor/256x256/apps/{package_name}.png` | 应用图标（高清） |
| `/usr/share/metainfo/{package_name}.metainfo.xml` | AppStream 元数据（如果配置了） |

这与 Google Chrome、VS Code、Discord 和 Zoom 等其他主流第三方 Linux 应用程序使用的约定相同。

## 相关链接

- [构建和发布 Linux 应用程序](https://docs.flutter.dev/deployment/linux)
- [RPM 打包指南](https://rpm-packaging-guide.github.io/)
- [patchelf](https://github.com/NixOS/patchelf)
