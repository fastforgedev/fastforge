# APT and Pacman Repositories

English | [简体中文](../../zh-Hans/publishers/repositories.md)

The `apt-repo` and `pacman-repo` targets add a package to a repository kept in a local directory and regenerate its index. Serve the directory from any static host, for example by syncing it with the [object storage](s3.md) publishers, `rsync`, or GitHub Pages, and users install and update the app with `apt` or `pacman`.

Each run adds one package and keeps the ones already there, so keep the directory between releases (for example, sync it down before publishing and back up afterwards).

## APT repository

Requires `dpkg-deb`, and `gpg` for signing.

```bash
fastforge publish --path dist/1.2.0+3/hello-1.2.0+3-linux.deb --target apt-repo \
  --publish-arg path=public/apt
```

The package is copied to `pool/<component>/<initial>/<package>/`, and `dists/<suite>/<component>/binary-<arch>/Packages` (and `Packages.gz`) and `dists/<suite>/Release` are regenerated from every package in the pool. Publishing a file name that is already in the pool with different contents fails, unless `replace=true` is given.

| Argument      | Description                                                         |
| ------------- | ------------------------------------------------------------------- |
| `path`        | Repository directory; may also use `APT_REPO_DIR`                   |
| `suite`       | Suite and codename; defaults to `stable`                            |
| `component`   | Component; defaults to `main`                                       |
| `origin`, `label` | Optional `Release` fields                                       |
| `signing-key` | gpg key ID to sign `Release` with; may also use `APT_SIGNING_KEY`   |
| `replace`     | `true` replaces a package file with the same name                   |

With a signing key, `InRelease` and `Release.gpg` are written. Users then add the repository with the exported public key:

```bash
curl -fsSL https://example.com/apt/key.gpg | sudo gpg --dearmor -o /usr/share/keyrings/hello.gpg
echo "deb [signed-by=/usr/share/keyrings/hello.gpg] https://example.com/apt stable main" | sudo tee /etc/apt/sources.list.d/hello.list
sudo apt update && sudo apt install hello
```

Without a key the repository is unsigned, and `apt` only accepts it with `[trusted=yes]`.

Launchpad PPAs only accept source packages and build them on Launchpad without network access, so they cannot take a packaged Flutter app; an APT repository is the way to offer `apt` updates.

## Pacman repository

Requires `repo-add` (from pacman), and `gpg` for signing.

```bash
fastforge publish --path dist/1.2.0+3/hello-1.2.0+3-linux.pkg.tar.zst --target pacman-repo \
  --publish-arg path=public/arch/x86_64 \
  --publish-arg repo=hello
```

The package is copied into the directory and `repo-add --remove` updates `<repo>.db` and `<repo>.files`, replacing and deleting an older version of the same package.

| Argument      | Description                                                          |
| ------------- | -------------------------------------------------------------------- |
| `path`        | Repository directory; may also use `PACMAN_REPO_DIR`                 |
| `repo`        | Repository name, as in `[repo]` of `pacman.conf` (required)          |
| `signing-key` | gpg key ID to sign the package and database; may also use `PACMAN_SIGNING_KEY` |

Users add it to `/etc/pacman.conf`:

```ini
[hello]
Server = https://example.com/arch/$arch
```

For the AUR, see [AUR](aur.md).
