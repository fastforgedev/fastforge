# Linux

English | [简体中文](../../zh-Hans/packagers/linux.md)

Fastforge packages Flutter Linux applications as [AppImage](#appimage), [DEB](#deb), [RPM](#rpm), [Pacman](#pacman), [ZIP, or a direct copy](#zip-and-direct).

## Current Status

| Build system    | `package` status                                              |
| --------------- | ------------------------------------------------------------- |
| Flutter Builder | AppImage, DEB, RPM, Pacman, ZIP, and direct through CLI/action |

Linux packaging requires a Flutter project (`pubspec.yaml`). The build runs only on a Linux host; on other hosts `fastforge package` skips the target with a warning. The packagers use the bundle directory `build/linux/<x64|arm64>/<mode>/bundle/`, and artifacts are written to `dist/<version>/`.

```bash
fastforge package --targets deb,appimage
```

The executable name is read from `set(BINARY_NAME "...")` in `linux/CMakeLists.txt`, falling back to the `pubspec.yaml` name.

## Configuration

AppImage, DEB, RPM, and Pacman are configured with the format's own files, placed in `linux/packaging/<format>/`. Fastforge renders them with its [packaging variables](../packaging.md#packaging-variables) and puts them into the package as they are. Each raw file replaces only the file Fastforge would otherwise generate, so you can start with a single file and leave the rest to the defaults.

| Format   | Raw files                                                                                                                   | Placed at                                 |
| -------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| DEB      | `control`, `postinst`, `postrm`; also `preinst`, `prerm`, `config`, `conffiles`, `triggers`, `templates`, `shlibs`, `symbols` | `DEBIAN/`                                  |
| RPM      | one `*.spec` file                                                                                                            | `SPECS/`                                   |
| Pacman   | `PKGINFO`, `INSTALL` (the dotted names `.PKGINFO`, `.INSTALL` also work)                                                    | `.PKGINFO`, `.INSTALL`                     |
| AppImage | `AppRun`                                                                                                                     | the AppDir root                            |
| All four | one `*.desktop` file                                                                                                         | the generated desktop entry's location    |
| All four | a `files/` directory                                                                                                         | copied into the package root (see below)  |

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

- **Rendering.** `${NAME}` is replaced only when `NAME` is a packaging variable. Everything else is kept, so shell expansions such as `$1` or `${HOME}`, RPM macros such as `%{buildroot}`, and desktop field codes such as `%U` need no escaping. Write `$${` for a literal `${`.
- **The `files/` overlay** is copied into the DEB and Pacman package root, the AppDir, or the RPM build directory. File and directory names are rendered too. Text files are rendered; binary files are copied unchanged. Permissions and symlinks are kept.
- **Generated behavior goes away with the file.** The generated `postinst` and Pacman `INSTALL` create the `/usr/bin/<binary>` symlink. A raw replacement has to do that itself if you want it, for example `ln -sf ${INSTALL_DIR}/${APP_BINARY_NAME} /usr/bin/${APP_BINARY_NAME}`.
- **RPM.** The bundle is staged in `${PACKAGING_DIRECTORY}/${APP_NAME}/` and the `files/` overlay in `${PACKAGING_DIRECTORY}/`, so a raw spec copies from there in `%install`. The variables are also set in rpmbuild's environment, so `%{getenv:APP_VERSION}` works as well. The artifact is taken from any `RPMS/<arch>/` directory, so the spec may set its own `BuildArch`.
- **Icon.** The icon comes from `icon` in `make_config.yaml`, or else from `project.icon` in `.fastforge/config.yaml`. It is installed as `usr/share/icons/hicolor/{128x128,256x256}/apps/<binary>` (DEB, Pacman), as `<binary>.<ext>` in the RPM build directory, and as `<app name>.<ext>` in the AppDir root, where `appimagetool` requires it.
- **Several `*.desktop` or `*.spec` files** in one format directory fail packaging, since Fastforge cannot tell which one to use.

`linux/packaging/<format>/make_config.yaml`, which follows the Dart CLI schema, is still read for compatibility and generates every file that has no raw counterpart. Common keys include `display_name`, `package_name`, `icon`, `metainfo`, `categories`, `keywords`, `generic_name`, and `startup_notify`. A missing file means defaults; a file that cannot be parsed fails packaging. The DEB default package name is the binary name, lowercased and with characters Debian does not allow, such as `_`, replaced by `-`.

## AppImage

Requires `appimagetool` and `ldd`. Additional `make_config.yaml` keys include `include` (extra shared objects, resolved with `locate`, which must then be installed) and `actions`.

## DEB

Requires `dpkg-deb` (`dpkg-dev` on Debian/Ubuntu). Additional `make_config.yaml` keys include `maintainer`, `co_authors`, `priority`, `section`, `dependencies`, and `postinstall_scripts`.

## RPM

Requires `rpmbuild` and `patchelf`. The generated spec works with rpm 4.20 and later, which run `%install` in a per-build directory. Additional `make_config.yaml` keys include `summary`, `group`, `vendor`, `packager`, `license`, `url`, `requires`, and `build_arch`.

## Pacman

Requires `bsdtar` and `xz`; `makepkg` is not used. The archive contains `.PKGINFO`, `.INSTALL`, and every top-level directory of the package root, including those from `files/`. Additional `make_config.yaml` keys include `maintainer`, `licenses`, `dependencies`, `optional_dependencies`, and `postinstall_scripts`.

## ZIP and direct

`zip` archives the bundle directory; `direct` copies it to `dist/<version>/` without an archive. Both need no extra tools.

Return to the [packager overview](README.md).
