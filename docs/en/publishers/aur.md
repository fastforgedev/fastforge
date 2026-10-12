# AUR

English | [简体中文](../../zh-Hans/publishers/aur.md)

The `aur` target publishes a `-bin` package to the [Arch User Repository](https://aur.archlinux.org). Its PKGBUILD downloads the pacman package Fastforge built and installs its files, so AUR users get the same package as the one published elsewhere.

## Requirements

- The [Pacman](../packagers/linux.md#pacman) package (`.pkg.tar.zst`), already uploaded somewhere public, such as a GitHub release.
- `bsdtar` and `git`.
- An AUR account whose SSH key the publishing machine uses for `aur.archlinux.org`.

## Publish

Pass the pacman package and the URL it can be downloaded from:

```bash
fastforge publish --path dist/1.2.0+3/hello-1.2.0+3-linux.pkg.tar.zst --target aur \
  --publish-arg source-url=https://github.com/owner/hello/releases/download/v1.2.0/hello-1.2.0+3-linux.pkg.tar.zst \
  --publish-arg maintainer="Jane Doe <jane at example dot com>"
```

Fastforge reads the package's `.PKGINFO` and writes a `PKGBUILD` and `.SRCINFO` with the same version, description, URL, licenses, dependencies, and relations, the checksum of the file, and the `.INSTALL` script when the package has one. The AUR package provides and conflicts with the original name. It then clones `ssh://aur@aur.archlinux.org/<package>.git`, commits the files, and pushes to `master`. An unchanged package is not committed again.

## Arguments

| Argument         | Description                                                                     |
| ---------------- | ------------------------------------------------------------------------------- |
| `source-url`     | Public URL of the same pacman package (required)                                |
| `package`        | AUR package name; defaults to `<pkgname>-bin`                                   |
| `maintainer`     | `# Maintainer:` line of the PKGBUILD                                            |
| `git-url`        | Repository to push to; defaults to the AUR's                                    |
| `commit-message` | Defaults to `Update to <pkgver>-<pkgrel>`                                       |
| `push`           | `false` commits without pushing and prints the checkout, to review it first     |

The result is the package's AUR page.
