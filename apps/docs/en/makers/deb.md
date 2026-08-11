# deb

Build your Flutter app as a Debian package (`.deb`) for installation on Debian-based Linux distributions such as Ubuntu, Debian, Linux Mint, and Pop!_OS. The DEB format is the standard package management format used by APT-based systems.

## Requirements

- Linux system (Debian/Ubuntu-based distribution recommended)
- Required tools: `dpkg`, `dpkg-deb` (typically pre-installed on Debian-based systems)

## Usage

Add `make_config.yaml` to your project `linux/packaging/deb` directory.

You can also add `make_config.yaml` to your project `linux/packaging` directory to inherit common configuration.

```yaml
display_name: Hello World
package_name: hello-world
maintainer:
  name: LiJianying
  email: lijy91@foxmail.com
co_authors:
  - name: Kingkor Roy Tirtho
    email: krtirtho@gmail.com
priority: optional
section: x11
installed_size: 6604
essential: false
icon: assets/logo.png

postinstall_scripts:
  - echo "Installed my awesome app"
postuninstall_scripts:
  - echo "Surprised Pickachu face"

keywords:
  - Hello
  - World
  - Test
  - Application

generic_name: Music Application

categories:
  - Music
  - Media

startup_notify: true
# You can also specify [metainfo](https://freedesktop.org/software/appstream/docs/chap-Quickstart.html) file
# which contains metadata of the app.
# metainfo: linux/packaging/myappid.appdata.xml
```

Run:

```
fastforge package --platform linux --targets deb
```

## Installation Structure

After installation, the application files are placed under `/opt/{package_name}/`, following the [Filesystem Hierarchy Standard (FHS)](https://refspecs.linuxfoundation.org/FHS_3.0/fhs/index.html) for third-party add-on software packages:

| Path | Description |
|---|---|
| `/opt/{package_name}/` | Application binaries and runtime files |
| `/usr/bin/{package_name}` | Symlink to the main binary (added to `$PATH`) |
| `/usr/share/applications/{package_name}.desktop` | Desktop entry for the application menu |
| `/usr/share/icons/hicolor/128x128/apps/{package_name}.png` | Application icon |
| `/usr/share/icons/hicolor/256x256/apps/{package_name}.png` | Application icon (high resolution) |
| `/usr/share/metainfo/{package_name}.metainfo.xml` | AppStream metadata (if configured) |

This follows the same convention used by other major third-party Linux applications such as Google Chrome, VS Code, Discord, and Zoom.

## Related Links

- [Build and release a Linux app](https://docs.flutter.dev/deployment/linux)
- [Packaging Debian packages, how it works](https://www.debian.org/doc/manuals/packaging-tutorial/packaging-tutorial.en.pdf)
