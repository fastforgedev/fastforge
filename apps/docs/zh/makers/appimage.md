# AppImage

将 Flutter 应用构建为 Linux AppImage 便携式应用格式。AppImage 无需安装即可在大多数 Linux 发行版上运行，它将应用及其依赖打包为单个可执行文件，提供"下载即运行"的体验。

## 环境要求

- 支持 FUSE 的 Linux 系统
- `locate` 工具用于依赖检测

  在基于 Ubuntu/Debian 的 Linux 上，运行：

  ```bash
  sudo apt install locate
  ```

- [appimagetool](https://github.com/AppImage/appimagetool)

  安装 appimagetool，运行：

  ```bash
  wget -O appimagetool "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x appimagetool
  mv appimagetool /usr/local/bin/
  ```

  > 最后一条命令可能需要 `sudo` 权限

## 使用方法

将 `make_config.yaml` 添加到您的项目 `linux/packaging/appimage` 目录。

你也可以将 `make_config.yaml` 添加到你的项目 `linux/packaging` 目录，以继承公共配置。

```yaml
display_name: Hello World

icon: assets/logo.png

keywords:
  - Hello
  - World
  - Test
  - Application

generic_name: Cool Application

actions:
  - name: Say Hi
    label: say-hi
    arguments:
      - --say
      - hi
  - name: Say Bye
    label: say-bye
    arguments:
      - --say
      - bye

categories:
  - Music

startup_notify: true

# 您可以指定要与您的应用捆绑的共享库
#
# fastforge 会自动检测您的应用所依赖的共享库，但您也可以在此处手动指定它们。
#
# 以下示例展示了如何将 libcurl 库与您的应用捆绑在一起
#
# include:
#   - libcurl.so.4
include: []
# 您也可以指定 [metainfo](https://www.freedesktop.org/software/appstream/metainfocreator/#/) 文件
# 其中包含应用的元数据。
# metainfo: linux/packaging/myappid.appdata.xml

# 注意：AppDir/lib 目录中的子目录（如 cmake/）会在依赖扫描时自动跳过，
# 避免 ldd 报错。
```

运行：

```bash
fastforge package --platform linux --targets appimage
```

## 相关链接

- [构建和发布 Linux 应用程序](https://docs.flutter.dev/deployment/linux)
- [AppImage 包格式介绍](https://docs.appimage.org/)
- [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html)
- [AppStream Metainfo 规范](https://www.freedesktop.org/software/appstream/docs/chap-Metadata.html)
