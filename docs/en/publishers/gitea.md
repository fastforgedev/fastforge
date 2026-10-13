# Gitea and Forgejo Releases

English | [简体中文](../../zh-Hans/publishers/gitea.md)

The `gitea` target uploads a file to a release on a Gitea server, including Forgejo and Codeberg, which share Gitea's release API. The release is found by name and created when it does not exist.

## Authentication

```bash
export GITEA_TOKEN=gitea-token
```

`GITEA_TOKEN` is required. Create an access token with the `write:repository` scope under **Settings → Applications** on the server.

## Publish

`--path` must point to a file, not a directory:

```bash
fastforge publish --path dist/app.zip --target gitea \
  --publish-arg server=https://codeberg.org \
  --publish-arg repo=owner/repository \
  --publish-arg release-title=v1.0.0
```

In Gitea or Forgejo Actions, the server and repository can come from the environment: `server` falls back to `GITEA_SERVER_URL`, then `GITHUB_SERVER_URL`, and `repo` to `GITEA_REPOSITORY`, then `GITHUB_REPOSITORY`. The server may be given with or without `/api/v1`, and may include a sub-path such as `https://example.com/git`.

## Release Selection

Releases are selected as for [GitHub Releases](github.md#release-selection): the title is `release-title` (with the same placeholders) or `v<app version>`, a release with that **name** is looked up among all releases, drafts included, and created with `release-tag` (or the title) as its tag when missing. Without a title or version, the file goes to the latest release.

## Arguments

| Argument             | Description                                                                 |
| -------------------- | --------------------------------------------------------------------------- |
| `server`             | Server URL, such as `https://codeberg.org`; may also use `GITEA_SERVER_URL` |
| `repo`               | `owner/repository`; may also use `GITEA_REPOSITORY`                         |
| `release-title`      | Name of the release to find or create                                       |
| `release-tag`        | Tag used only when creating a release                                       |
| `release-draft`      | `true` or `1` creates a draft (applies only when creating)                  |
| `release-prerelease` | `true` or `1` creates a prerelease (applies only when creating)             |
| `app-version`        | Version used for the default title; same as `--app-version`                 |

## Result

The publishing result is the asset's download URL. Whether an asset with the same file name is rejected depends on the server.
