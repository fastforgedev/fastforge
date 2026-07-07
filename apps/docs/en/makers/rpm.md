# rpm

Build your Flutter app as an RPM package for installation on Red Hat-based Linux distributions such as Fedora, RHEL, CentOS, and OpenSUSE. RPM is the standard package management format used by these distributions.

## Requirements

- Linux system (Fedora/RHEL-based distribution recommended)
- [patchelf](https://github.com/NixOS/patchelf) — for modifying ELF binaries
- [rpmbuild](https://rpm-packaging-guide.github.io/#prerequisites) — RPM package builder

Install requirements:

- Debian/Ubuntu: `apt install rpm patchelf`
- Fedora: `dnf install gcc rpm-build rpm-devel rpmlint make python bash coreutils diffutils patch rpmdevtools patchelf`
- Arch: `yay -S rpmdevtools patchelf` or `pamac install rpmdevtools patchelf`

## Usage

Add `make_config.yaml` to your project `linux/packaging/rpm` directory.

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

# RPM spec macros — inject global RPM directives at the top of the .spec file
#
# Useful for solving cross-distro ABI conflicts, excluding automatic requires,
# or setting build-id options. Each entry is placed verbatim at the top of the
# generated .spec file, before the preamble.
#
# spec_macros:
#   - "%global __requires_exclude .*libcurl.*CURL_OPENSSL.*"
#   - "%define _build_id_links none"

# You can also specify [metainfo](https://freedesktop.org/software/appstream/docs/chap-Quickstart.html) file
# which contains metadata of the app.
# metainfo: linux/packaging/myappid.appdata.xml
```

Run:

```
fastforge package --platform linux --targets rpm
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
- [RPM Packaging Guide](https://rpm-packaging-guide.github.io/)
- [patchelf](https://github.com/NixOS/patchelf)
