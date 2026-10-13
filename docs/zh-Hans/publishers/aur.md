# AUR

[English](../../en/publishers/aur.md) | 简体中文

`aur` 目标把 `-bin` 软件包发布到 [Arch 用户软件仓库（AUR）](https://aur.archlinux.org)。其 PKGBUILD 下载 Fastforge 构建的 pacman 软件包并安装其中的文件，因此 AUR 用户得到的软件包与其他渠道发布的一致。

## 前提

- [Pacman](../packagers/linux.md#pacman) 软件包（`.pkg.tar.zst`），并已上传到公开位置，例如 GitHub Release。
- `bsdtar` 和 `git`。
- AUR 账号，且发布机器访问 `aur.archlinux.org` 时使用该账号的 SSH 密钥。

## 发布

传入 pacman 软件包及其下载地址：

```bash
fastforge publish --path dist/1.2.0+3/hello-1.2.0+3-linux.pkg.tar.zst --target aur \
  --publish-arg source-url=https://github.com/owner/hello/releases/download/v1.2.0/hello-1.2.0+3-linux.pkg.tar.zst \
  --publish-arg maintainer="Jane Doe <jane at example dot com>"
```

Fastforge 读取软件包的 `.PKGINFO`，生成 `PKGBUILD` 和 `.SRCINFO`，沿用其版本、描述、URL、许可证、依赖和关联关系，写入文件校验和，软件包含 `.INSTALL` 时一并带上。AUR 软件包会 provides 并 conflicts 原名称。随后克隆 `ssh://aur@aur.archlinux.org/<package>.git`，提交文件并推送到 `master`。内容未变化时不会重复提交。

## 参数

| 参数             | 说明                                                       |
| ---------------- | ---------------------------------------------------------- |
| `source-url`     | 同一 pacman 软件包的公开下载地址（必填）                   |
| `package`        | AUR 软件包名，默认 `<pkgname>-bin`                         |
| `maintainer`     | PKGBUILD 中的 `# Maintainer:` 行                           |
| `git-url`        | 推送的仓库，默认为 AUR 仓库                                |
| `commit-message` | 默认 `Update to <pkgver>-<pkgrel>`                         |
| `push`           | `false` 时只提交不推送，并输出本地检出目录，便于先行检查   |

发布结果为该软件包的 AUR 页面。
