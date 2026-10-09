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

Fastforge builds every Linux package in two steps. First it stages the **package root**, everything that gets installed, at its installed path:

- the bundle in `/opt/<binary>` (the root itself for an AppImage) and a `/usr/bin/<binary>` link to the binary;
- the desktop entry, `${APP_ID}.desktop`;
- the icon, as `${APP_ID}`, in the hicolor theme;
- the AppStream metainfo, `${APP_ID}.metainfo.xml`;
- the `files/` overlays.

Then it adds the format's metadata: `DEBIAN/control`, the RPM spec, `.PKGINFO`, or `AppRun`. Both steps can be customized with the format's own files in `.fastforge/packaging/linux/`, rendered with the [packaging variables](../packaging.md#packaging-variables). Each raw file replaces only the file Fastforge would otherwise generate, so you can start with one file and leave the rest to the defaults.

| Directory  | Raw files                                                                                                   |
| ---------- | ----------------------------------------------------------------------------------------------------------- |
| `shared/`  | one `*.desktop` file, one `*.metainfo.xml` file, and a `files/` overlay, used by every format              |
| `deb/`     | `control`, `preinst`, `postinst`, `prerm`, `postrm`, `config`, `conffiles`, `triggers`, `templates`, `shlibs`, `symbols` |
| `rpm/`     | one `*.spec` file, under any name                                                                           |
| `pacman/`  | `PKGINFO`, `INSTALL` (`.PKGINFO` and `.INSTALL` work too)                                                    |
| `appimage/` | `AppRun`                                                                                                   |

Every format directory may also hold its own `*.desktop` file, `*.metainfo.xml` file, and `files/` overlay, which win over the shared ones.

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

[`examples/hello_world`](../../../examples/hello_world/.fastforge/packaging/linux/) uses this layout without any `make_config.yaml`.

Project metadata shared by the formats goes in `project:` in `.fastforge/config.yaml`; see [Packaging variables](../packaging.md#packaging-variables):

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

- **Rendering.** `${NAME}` is replaced only when `NAME` is a packaging variable. Everything else is kept, so shell expansions such as `$1` or `${HOME}`, RPM macros such as `%{buildroot}`, and desktop field codes such as `%U` need no escaping. Write `$${` for a literal `${`. Overlay file and directory names are rendered too, such as `files/usr/share/doc/${PACKAGE_NAME}/copyright`.
- **Overlays.** The shared `files/` is copied into the package root first, then the format's own, whose files win. An AppImage only takes the shared overlay's `usr/`, since it installs nothing in `/etc` or `/opt`. Text files are rendered; binary files are copied unchanged. Permissions and symlinks are kept.
- **Filled in automatically.** DEB gets `Installed-Size` when the control file has none, `md5sums`, and every `/etc` file in `conffiles`. Pacman gets `size`, `builddate`, and a `backup` entry for every `/etc` file. RPM lists `/etc` files as `%config(noreplace)`. Values you write yourself are kept.
- **Desktop entry and metainfo.** The templates can have any name: one `*.desktop` file and one `*.metainfo.xml` (or `*.appdata.xml`) file. Whatever their names, they are installed as `${APP_ID}.desktop` and `usr/share/metainfo/${APP_ID}.metainfo.xml`, as the desktop entry and AppStream specifications expect. In the desktop entry, refer to the program as `${APP_BINARY_NAME}` and to the icon as `${APP_ID}`. On Linux `APP_ID` always has a value: `project.app_id`, else `APPLICATION_ID` from `linux/CMakeLists.txt`, else the binary name.
- **Wayland.** The desktop file is matched to the window's app ID, and GTK reports the program name as the app ID. Keep `APPLICATION_ID` in `linux/CMakeLists.txt` equal to `project.app_id`, or leave `project.app_id` unset. Also call `g_set_prgname(APPLICATION_ID)` in `my_application_new()`, as current Flutter templates do; otherwise the window reports the binary name.
- **Icon.** The icon is `project.icon`, unless a `make_config.yaml` sets `icon`. An SVG is installed as scalable. A PNG is padded to a square and resized to every standard size up to its own, from 16 to 512 px. Other formats fail packaging. To ship hand-drawn sizes, put them in a `files/` overlay instead.
- **Package name and version.** `PACKAGE_NAME` is `package_name` from `make_config.yaml`, else `project.package_name`, else the format's default. `PACKAGE_VERSION` is the version in the format's syntax, with pre-releases such as `1.2.3-beta.1` ordered before the release.
- **Several templates of one kind** in one directory, such as two `*.desktop` files or two `*.spec` files, fail packaging, since Fastforge cannot tell which one to use.

For projects that also use the Dart CLI, `linux/packaging/<format>/make_config.yaml` is still read and generates every file that has no raw counterpart; new projects do not need it. Common keys include `display_name`, `package_name`, `icon`, `metainfo`, `categories`, `keywords`, `generic_name`, and `startup_notify`. A missing file means defaults; a file that cannot be parsed fails packaging. Without a raw file or `make_config.yaml`, the generated metadata comes from `project:`, and a missing license is written as `LicenseRef-Unknown`.

## AppImage

Requires `appimagetool` and `ldd`. An AppImage runs on systems whose glibc is at least as new as the build host's, so build it on the oldest distribution you support. The default `AppRun` puts the bundled libraries first and starts the binary with the given arguments, in the current directory. Shared objects the plugins need are copied into `usr/lib`, unless a `files/` overlay already provides them. Additional `make_config.yaml` keys include `include` (extra shared objects, resolved with `locate`, which must then be installed) and `actions`.

## DEB

Requires `dpkg-deb` (`dpkg-dev` on Debian/Ubuntu). Files are owned by root. The default package name is the binary name, lowercased, with characters Debian does not allow, such as `_`, replaced by `-`. `Version` keeps the build number (`1.2.3+4`) and writes a pre-release's `-` as `~`. Without `dependencies` in `make_config.yaml`, the package depends on `libgtk-3-0`; an empty list opts out. Maintainer scripts are only written when you provide them or set `postinstall_scripts`/`postuninstall_scripts`, since the `/usr/bin` link is part of the package. Additional `make_config.yaml` keys include `maintainer`, `co_authors`, `priority`, `section`, and `dependencies`.

## RPM

Requires `rpmbuild` and `patchelf`. The package root is at `${PACKAGING_DIRECTORY}` and its `%files` list at `${RPM_FILE_LIST}`, so a spec installs it in two lines. The spec is written as `SPECS/${PACKAGE_NAME}.spec`, whatever its own name.

```spec
%global __provides_exclude_from ^${INSTALL_DIR}/.*$
%global __requires_exclude ^(${RPM_PRIVATE_LIBS})

%install
cp -a ${PACKAGING_DIRECTORY}/. %{buildroot}/

%files -f ${RPM_FILE_LIST}
```

The list owns the files, links, and every directory the package creates, such as `/opt/<binary>` or `/etc/<app id>`, but not system directories such as `/usr/share/applications`. The two `%global` lines keep the bundled Flutter engine and plugins out of the automatic `Provides` and `Requires`. System libraries such as GTK and glibc stay required. `RPM_PRIVATE_LIBS` lists the bundle's libraries, escaped for a spec. The generated spec does this too, and turns off debuginfo packages and `.build-id` links, which would clash between apps that ship the same engine. `Version` is the build name, with `-` turned into `~`, and `Release` is the build number. The variables are also set in rpmbuild's environment, so `%{getenv:APP_VERSION}` works. The artifact is taken from any `RPMS/<arch>/` directory, so a spec may set its own `BuildArch`. Additional `make_config.yaml` keys include `summary`, `group`, `vendor`, `packager`, `license`, `url`, `requires`, and `build_arch`.

## Pacman

Requires `bsdtar` with zstd support; `makepkg` is not used. The artifact is a `.pkg.tar.zst` archive owned by root, with `.MTREE`, `.PKGINFO`, `.INSTALL` when there is one, and the package root. The generated `.PKGINFO` uses makepkg's format, one `key = value` line per value. `pkgver` is `<build name>-<build number>`, with `-` removed from the build name. Without `dependencies` in `make_config.yaml`, the package depends on `gtk3`; an empty list opts out. Additional `make_config.yaml` keys include `maintainer`, `licenses`, `groups`, `dependencies`, `optional_dependencies`, and `postinstall_scripts`.

## ZIP and direct

`zip` archives the bundle directory; `direct` copies it to `dist/<version>/` without an archive. Both need no extra tools.

Return to the [packager overview](README.md).
