# appimage

Build your Flutter app as a Linux AppImage — a portable application format that runs on most Linux distributions without installation. AppImages bundle the application and its dependencies into a single executable file, providing a "download and run" experience.

## Requirements

- Linux system with FUSE support
- `locate` utility for dependency detection

  On Ubuntu/Debian based Linux, run:

  ```bash
  sudo apt install locate
  ```

- [appimagetool](https://github.com/AppImage/appimagetool)

  To install appimagetool, run:

  ```bash
  wget -O appimagetool "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x appimagetool
  mv appimagetool /usr/local/bin/
  ```

  > The last command may require `sudo` privileges

## Usage

Add `make_config.yaml` to your project `linux/packaging/appimage` directory.

You can also add `make_config.yaml` to your project `linux/packaging` directory to inherit common configuration.

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

# You can specify the shared libraries that you want to bundle with your app
#
# fastforge automatically detects the shared libraries that your app
# depends on, but you can also specify them manually here.
#
# The following example shows how to bundle the libcurl library with your app.
#
# include:
#   - libcurl.so.4
include: []
# You can also specify [metainfo](https://www.freedesktop.org/software/appstream/metainfocreator/#/) file
# which contains metadata of the app.
# metainfo: linux/packaging/myappid.appdata.xml

# Note: Directories inside the AppDir/lib folder (e.g. cmake/) are automatically
# skipped during dependency scanning, preventing ldd errors.
```

Run:

```bash
fastforge package --platform linux --targets appimage
```

## Related Links

- [Build and release a Linux app](https://docs.flutter.dev/deployment/linux)
- [Introduction to AppImage package format](https://docs.appimage.org/)
- [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html)
- [AppStream Metainfo Specification](https://www.freedesktop.org/software/appstream/docs/chap-Metadata.html)
