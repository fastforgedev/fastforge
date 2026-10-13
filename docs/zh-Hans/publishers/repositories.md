# APT 与 Pacman 软件源

[English](../../en/publishers/repositories.md) | 简体中文

`apt-repo` 和 `pacman-repo` 目标把软件包加入保存在本地目录中的软件源，并重新生成索引。用任意静态托管提供该目录（例如通过[对象存储](s3.md)发布器、`rsync` 或 GitHub Pages 同步），用户即可用 `apt` 或 `pacman` 安装和更新应用。

每次运行只添加一个包，并保留已有的包，因此请在各次发布之间保留该目录（例如发布前先同步下来，发布后再同步回去）。

## APT 软件源

需要 `dpkg-deb`，签名时还需要 `gpg`。

```bash
fastforge publish --path dist/1.2.0+3/hello-1.2.0+3-linux.deb --target apt-repo \
  --publish-arg path=public/apt
```

软件包复制到 `pool/<component>/<首字母>/<package>/`，并根据池中全部软件包重新生成 `dists/<suite>/<component>/binary-<arch>/Packages`（及 `Packages.gz`）和 `dists/<suite>/Release`。池中已有同名但内容不同的文件时发布失败，除非传入 `replace=true`。

| 参数          | 说明                                                      |
| ------------- | --------------------------------------------------------- |
| `path`        | 软件源目录；也可用 `APT_REPO_DIR`                         |
| `suite`       | suite 与 codename，默认 `stable`                          |
| `component`   | 组件，默认 `main`                                         |
| `origin`、`label` | 可选的 `Release` 字段                                 |
| `signing-key` | 用于签名 `Release` 的 gpg 密钥 ID；也可用 `APT_SIGNING_KEY` |
| `replace`     | `true` 时替换同名软件包文件                               |

提供签名密钥时会写入 `InRelease` 和 `Release.gpg`。用户使用导出的公钥添加软件源：

```bash
curl -fsSL https://example.com/apt/key.gpg | sudo gpg --dearmor -o /usr/share/keyrings/hello.gpg
echo "deb [signed-by=/usr/share/keyrings/hello.gpg] https://example.com/apt stable main" | sudo tee /etc/apt/sources.list.d/hello.list
sudo apt update && sudo apt install hello
```

没有密钥时软件源不签名，`apt` 只在 `[trusted=yes]` 时接受。

Launchpad PPA 只接受源码包，并在无网络的 Launchpad 构建机上构建，因此无法接收打包好的 Flutter 应用；要通过 `apt` 提供更新，请使用 APT 软件源。

## Pacman 软件源

需要 `repo-add`（pacman 提供），签名时还需要 `gpg`。

```bash
fastforge publish --path dist/1.2.0+3/hello-1.2.0+3-linux.pkg.tar.zst --target pacman-repo \
  --publish-arg path=public/arch/x86_64 \
  --publish-arg repo=hello
```

软件包复制到该目录，`repo-add --remove` 更新 `<repo>.db` 和 `<repo>.files`，并替换、删除同一软件包的旧版本。

| 参数          | 说明                                                       |
| ------------- | ---------------------------------------------------------- |
| `path`        | 软件源目录；也可用 `PACMAN_REPO_DIR`                       |
| `repo`        | 软件源名称，即 `pacman.conf` 中的 `[repo]`（必填）         |
| `signing-key` | 用于签名软件包和数据库的 gpg 密钥 ID；也可用 `PACMAN_SIGNING_KEY` |

用户将其加入 `/etc/pacman.conf`：

```ini
[hello]
Server = https://example.com/arch/$arch
```
