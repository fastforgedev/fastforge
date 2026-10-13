# Gitea 与 Forgejo Releases

[English](../../en/publishers/gitea.md) | 简体中文

`gitea` 目标把文件上传到 Gitea 服务器上的 Release，同样适用于共用 Gitea Release API 的 Forgejo 和 Codeberg。Release 按名称查找，不存在时自动创建。

## 认证

```bash
export GITEA_TOKEN=gitea-token
```

`GITEA_TOKEN` 为必填项。在服务器的 **设置 → 应用** 中创建带 `write:repository` 权限的访问令牌。

## 发布

`--path` 必须指向文件，不能是目录：

```bash
fastforge publish --path dist/app.zip --target gitea \
  --publish-arg server=https://codeberg.org \
  --publish-arg repo=owner/repository \
  --publish-arg release-title=v1.0.0
```

在 Gitea 或 Forgejo Actions 中，服务器和仓库可以取自环境变量：`server` 依次回退到 `GITEA_SERVER_URL`、`GITHUB_SERVER_URL`，`repo` 依次回退到 `GITEA_REPOSITORY`、`GITHUB_REPOSITORY`。服务器地址可以带或不带 `/api/v1`，也可以包含子路径，例如 `https://example.com/git`。

## Release 选择

与 [GitHub Releases](github.md#release-的选择方式) 相同：标题取 `release-title`（支持相同的占位符），否则为 `v<应用版本>`；在全部 Release（包括草稿）中查找**名称**等于标题的 Release，不存在时以 `release-tag`（缺省为标题）作为 tag 创建。既没有标题也没有版本时，上传到最新的 Release。

## 参数

| 参数                 | 说明                                                         |
| -------------------- | ------------------------------------------------------------ |
| `server`             | 服务器地址，例如 `https://codeberg.org`；也可用 `GITEA_SERVER_URL` |
| `repo`               | `owner/repository`；也可用 `GITEA_REPOSITORY`                |
| `release-title`      | 要查找或创建的 Release 名称                                  |
| `release-tag`        | 仅在创建 Release 时使用的 tag                                |
| `release-draft`      | `true` 或 `1` 时创建草稿（仅创建时生效）                     |
| `release-prerelease` | `true` 或 `1` 时创建预发布（仅创建时生效）                   |
| `app-version`        | 用于默认标题的版本，等同于 `--app-version`                   |

## 结果

发布结果为附件的下载地址。同名附件是否会被拒绝取决于服务器。
