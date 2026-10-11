# Google Play

English | [简体中文](../../zh-Hans/stores/googleplay.md)

`fastforge googleplay` operates the Google Play Developer API directly, covering app verification, edits, AAB uploads, tracks, and catalogs.

## Authentication

`GOOGLE_PLAY_SERVICE_ACCOUNT_JSON` may contain complete service-account JSON or a file path:

```bash
export GOOGLE_PLAY_SERVICE_ACCOUNT_JSON="$PWD/service-account.json"
```

The service account needs Google Play Developer API access to the target app.

## Apps

```bash
fastforge googleplay app view com.example.myapp
fastforge googleplay app check com.example.myapp
```

`app view` prints the package name and Play Console URL without calling the API. `app check` verifies access by creating and deleting an edit.

## Edit Workflow

Most write operations take place within an edit:

```bash
fastforge googleplay edit create \
  --package-name com.example.myapp

fastforge googleplay edit commit \
  --package-name com.example.myapp \
  --edit-id <edit-id>
```

Use `edit delete` when an edit is no longer needed.

## Upload an AAB

Reuse an existing edit:

```bash
fastforge googleplay bundle upload dist/app-release.aab \
  --package-name com.example.myapp \
  --edit-id <edit-id>
```

You can also select a track and commit immediately after the upload:

```bash
fastforge googleplay bundle upload dist/app-release.aab \
  --package-name com.example.myapp \
  --track internal \
  --release-name '1.0.0 (1)' \
  --commit
```

- Only `.aab` files are accepted.
- Without `--edit-id`, a new edit is created. Without `--commit`, that edit stays uncommitted; the output includes its edit ID.
- Without `--track`, the bundle is uploaded but not assigned to a track.
- `--status` defaults to `completed`. `--release-name` defaults to the AAB file name without the `.aab` extension.

## Tracks

```bash
fastforge googleplay track list \
  --package-name com.example.myapp \
  --edit-id <edit-id>

fastforge googleplay track view internal \
  --package-name com.example.myapp \
  --edit-id <edit-id>

fastforge googleplay track update internal \
  --package-name com.example.myapp \
  --edit-id <edit-id> \
  --version-code 1 \
  --status completed

fastforge googleplay edit commit \
  --package-name com.example.myapp \
  --edit-id <edit-id>
```

Track names include `internal`, `alpha`, `beta`, `production`, and custom tracks. `track update` never commits the edit, so always follow it with `edit commit`.

`track update` and `bundle upload --track` replace the track's releases with a single release that contains only the release name, version code, and status. `--release-name` defaults to `release <version-code>` for `track update`. There are no options for a rollout fraction (`userFraction`) or release notes, so staged rollouts and release notes require `fastforge googleplay api` or a track YAML file pushed through the [catalog](catalog.md).

## Catalog and Raw API

- See [Unified Catalog](catalog.md) for listing, image, and track metadata synchronization.
- Call endpoints without typed commands through `fastforge googleplay api`:

```bash
fastforge googleplay api get /applications/com.example.myapp/edits/<edit-id>/tracks
fastforge googleplay api put /applications/com.example.myapp/edits/<edit-id>/tracks/beta \
  --input track.json
```

Supported methods are `get`, `post`, `put`, `patch`, and `delete`. The path is relative to `androidpublisher/v3` and must start with `/applications/`, `--query KEY=VALUE` can be repeated, and `post`/`put`/`patch` accept `--input` pointing to a JSON request body file.
