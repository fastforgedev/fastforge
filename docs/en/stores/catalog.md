# Unified Catalog

English | [简体中文](../../zh-Hans/stores/catalog.md)

Catalog synchronization stores App Store and Google Play metadata and images in a local directory for version control, bulk editing, and repeatable publishing. AppGallery does not have catalog synchronization.

## Bulk Synchronization

First register apps in `.fastforge/config.yaml`, then run:

```bash
fastforge store catalog pull
fastforge store catalog push
```

- The command processes every App Store app in configuration order, then every Google Play app. AppGallery apps appear in `fastforge store list` but are skipped by catalog commands.
- App Store entries use `bundle_id` (falling back to `app_id`) and `apps[].platform`; older configurations without `platform` default to `IOS`.
- Bulk commands always use the default directories, and `fastforge store catalog push` has no `--dry-run`: it pushes immediately.
- A failure for one application does not stop subsequent applications, but the process exits with an error if any application failed.
- Two App Store entries with the same bundle ID but different platforms share one directory. Each pull replaces that directory, so only the last entry's platform remains. Pull such platforms separately with different `--output` roots.

## Default Directory

```text
.fastforge/stores/
├── appstore/
│   └── com.example.myapp/
└── googleplay/
    └── com.example.myapp/
```

## Synchronize App Store Separately

```bash
fastforge appstore catalog pull \
  --app com.example.myapp \
  --platform MAC_OS \
  --version 1.0.0 \
  --output .fastforge/stores/appstore

fastforge appstore catalog push \
  --app com.example.myapp \
  --input .fastforge/stores/appstore \
  --dry-run
```

`--app` accepts the same identifiers as `appstore app view`. `--output` and `--input` are roots; the resolved bundle ID directory is appended. `--platform` defaults to `IOS`, and omitting `--version` pulls every version for that platform.

Catalog pull stages a complete snapshot beside the destination and replaces the entire `<bundle-id>/` directory only after every required request and write succeeds. A failed pull discards its staged files and preserves the previous snapshot. A successful pull removes anything not in the new snapshot, including other platforms, other versions, and uncommitted local edits. A failed screenshot or preview download only prints a warning and leaves that file out, and media that has no download URL yet is written as an empty placeholder file; check for both before pushing. When using older or unofficial binaries without this behavior, a failed pull may leave partial files: do not push, and first confirm the expected version directories and `.manifest.yaml` (when screenshots exist), or pull again into a new directory.

Directory structure:

```text
<bundle-id>/
├── app.yaml
├── app_info.yaml
├── info/
│   └── en-US.yaml
├── versions/
│   └── IOS/
│       └── 1.0.0/
│           ├── version.yaml
│           ├── review.yaml
│           ├── review.d/
│           │   └── <attachment-id>.yaml
│           └── en-US/
│               ├── localization.yaml
│               ├── screenshots/
│               │   └── <DISPLAY_TYPE>/001.png
│               └── previews/
│                   └── <PREVIEW_TYPE>/001_<id>.mov
└── .manifest.yaml
```

- `app.yaml` records the bundle ID, name, primary locale, and SKU; it is not pushed.
- `app_info.yaml` manages primary, secondary, and subcategories.
- `info/<locale>.yaml` stores app-info localizations such as name, subtitle, and privacy policy URL.
- `version.yaml` stores version-level fields (`_id`, `platform`, `versionString`, `state`, `copyright`).
- `review.yaml` stores App Review details; `review.d/` stores review attachment metadata.
- `localization.yaml` stores locale-specific fields such as description, keywords, what's new, promotional text, and URLs.
- `.manifest.yaml` stores remote screenshot IDs and checksums for each screenshot set.

Pull removes duplication across versions. For each platform, versions are processed from oldest to newest, and localization fields, copyright, `review.yaml`, and screenshot or preview sets are written only when they differ from the previous version. Sparse version directories are expected.

Push behavior:

- `app_info.yaml` and `info/<locale>.yaml` are updated; missing locales are created. Remote locales are never deleted.
- Only `copyright` from `version.yaml` is pushed. A version directory without a matching remote version is skipped; push does not create versions.
- `localization.yaml` updates changed fields and creates missing locales. Remote locales are never deleted.
- Screenshots are synchronized per display-type directory that contains files (PNG or JPEG only; any other non-hidden file fails the push). Missing screenshot sets are created: unchanged files are reused by checksum, new files are uploaded, remote screenshots without a local file are deleted, and order follows local file names. An empty or missing directory leaves the remote set unchanged.
- Previews are pulled but not pushed.
- `review.yaml` is updated or created. `review.d/` only updates existing attachments; new attachments are not uploaded.

## Synchronize Google Play Separately

```bash
fastforge googleplay catalog pull \
  --package-name com.example.myapp \
  --output .fastforge/stores/googleplay

fastforge googleplay catalog push \
  --package-name com.example.myapp \
  --input .fastforge/stores/googleplay \
  --dry-run
```

`--output` and `--input` are roots; the package name directory is appended.

Directory structure:

```text
<package-name>/
├── app.yaml
├── listings/
│   └── <language>.yaml
├── screenshots/
│   └── <language>/
│       └── <type>/NNN_<image-id>.<ext>
└── tracks/
    └── <track>.yaml
```

- `<type>` is one of `phone_screenshots`, `seven_inch_screenshots`, `ten_inch_screenshots`, `tv_screenshots`, `wear_screenshots`, `feature_graphic`, `tv_banner`, or `icon`.
- Colons in track names become underscores in file names (for example, `wear:production` becomes `wear_production.yaml`).
- Google Play pull writes into the existing directory and never removes files. Images deleted remotely remain locally and would be uploaded again on push, so pull into an empty directory when you need an exact snapshot.

Push runs in a single edit that is committed automatically at the end:

- `listings/*.yaml` are created or updated. Remote languages are never deleted.
- For each image-type directory that contains files, all remote images of that type and language are deleted, then local files are uploaded in file-name order. An individual upload failure is only reported as a warning, so a set can end up incomplete. Empty or missing directories leave remote images unchanged.
- Every `tracks/*.yaml` file is sent as-is and replaces that track's releases, including status, `userFraction`, and release notes. A stale track file can change live releases; remove track files you do not intend to change before pushing.

## Safe Operating Sequence

1. Run `pull` to retrieve the latest remote state. For App Store, pull each platform and version you need, using separate roots if they would share a directory. For Google Play, pull into an empty directory.
2. Edit YAML and images on a separate branch.
3. Review the diff. Removing a screenshot from a non-empty directory deletes it remotely; removing a whole locale or image directory deletes nothing remotely.
4. Preview with the individual store command's `push --dry-run`. Dry run lists the planned actions for each file, not a field-level diff; App Store lists every `info/` locale and version `localization.yaml` as `create` even when it already exists.
5. Run the actual push after confirmation.
