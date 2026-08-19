# CLI

How to use the command line interface (CLI) for Fastforge

## Installation

```shell
dart pub global activate fastforge
```

> **Windows users:** After activation, ensure the pub cache bin directory is in your PATH:
> 1. Open **System Properties** → **Advanced** → **Environment Variables**
> 2. Under **User variables**, select `Path` → **Edit**
> 3. Add `%APPDATA%\Pub\Cache\bin` and click **OK**
> 4. Restart your terminal, then try `fastforge --help`

---

## Commands

> These commands are sorted in alphabetical order. The most commonly used are package, publish, and release.

### Package

Will package your application into a platform specific format and put the result in a folder.

| Flag | Value | Required |
|------|-------|:--------:|
| `--platform` | Platform, e.g. `android` (auto-detected from the targets and project layout when omitted) | ❌ |
| `--targets` | Comma separated list of maker names | ✅ |
| `--skip-clean` | Skip clean once before build | ❌ |
| `--hook-pre` | Shell command to run before packaging | ❌ |
| `--hook-post` | Shell command to run after packaging | ❌ |

Example:

```shell
fastforge package --platform=android --targets=aab,apk

# The platform can be omitted: `aab`/`apk` only belong to android
fastforge package --targets=aab,apk

fastforge package --platform=macos --target=zip --hook-pre 'echo "before"' --hook-post 'echo "after"'
```

> `--platform` is optional. Most targets map to exactly one platform (e.g. `dmg` → macos, `apk` → android); ambiguous targets such as `zip`, `direct` and `custom` are resolved from the platform directories in your project and the host OS. You only need to specify it when no unambiguous platform can be determined.

### Publish

| Flag | Value | Required |
|------|-------|:--------:|
| `--path` | Path, e.g. `hello_world-1.0.0+1-android.apk` | ✅ |
| `--targets` | Comma separated list of publisher names | ✅ |

Example:

```shell
fastforge publish --path hello_world-1.0.0+1-android.apk --targets fir,pgyer
```

### Release

Will according to the configuration file (`distribute_options.yaml`), package your application into a specific format and publish it to the distribution platform.

| Flag | Value | Required |
|------|-------|:--------:|
| `--name` | Name, e.g. `dev` | ✅ |
| `--skip-clean` | Skip clean once before build | ❌ |

Example:

```shell
fastforge release --name dev
```

### App Store review submissions

The `appstore submission` commands manage the current App Store Connect review
submission workflow. They use `APP_STORE_CONNECT_KEY_ID`,
`APP_STORE_CONNECT_ISSUER_ID`, and `APP_STORE_CONNECT_KEY_PATH` for
authentication.

```shell
# Inspect submissions and their items
fastforge appstore submission list --app com.example.myapp
fastforge appstore submission view <submission-id>
fastforge appstore submission items <submission-id>

# Create a draft, add an App Store version, and submit it
fastforge appstore submission create --app com.example.myapp --platform IOS
fastforge appstore submission add-item <submission-id> \
  --item-type appStoreVersions --item-id <version-id>
fastforge appstore submission submit <submission-id> --wait

# Remove a draft item or cancel a submitted review
fastforge appstore submission remove-item <submission-item-id>
fastforge appstore submission cancel <submission-id>
```

`list` accepts optional `--platform` and `--state` filters. `add-item` also
supports App Store Connect reviewable resource types for custom product page
versions, version experiments, app events, background assets, and Game Center
versions. Use `--json` with any read or mutation command to produce structured
output.

`fastforge appstore version submit <version> --app <app> --build <build>` uses
this review-submission workflow automatically: it attaches the build, creates a
submission, adds the App Store version as an item, and submits it for review.

### Store catalog

Pull or push catalog data for every App Store and Google Play app configured in
`.fastforge/config.yaml`:

```yaml
stores:
  appstore:
    apps:
      - bundle_id: com.example.myapp
        app_id: "1234567890" # Optional fallback when bundle_id is omitted
  googleplay:
    apps:
      - package_name: com.example.myapp
```

```shell
fastforge store catalog pull
fastforge store catalog push
```

The unified command reads app identifiers from the configuration file and uses
the existing authentication environment variables:

| Store | Environment variables |
|-------|-----------------------|
| App Store | `APP_STORE_CONNECT_KEY_ID`, `APP_STORE_CONNECT_ISSUER_ID`, `APP_STORE_CONNECT_KEY_PATH` |
| Google Play | `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON` (service account JSON or a JSON file path) |

Catalog files use the existing default directories under
`.fastforge/stores/appstore/` and `.fastforge/stores/googleplay/`. All configured
apps are processed in order. If one app fails, the remaining apps continue and
the command exits with an error after printing a summary.

For App Store versions, `pull` writes version-level attributes such as
`copyright` to `versions/<platform>/<version>/version.yaml`. Localized attributes
are written to
`versions/<platform>/<version>/<locale>/localization.yaml`. Editing these files
and running `push` updates the corresponding App Store Connect resources.
Unchanged copyright values are omitted from consecutive versions of the same
platform. Push still accepts the legacy locale-level `version.yaml`, but reports
an error when both localization filenames exist in the same locale directory.

App-level App Store categories are stored in `<bundle-id>/app_info.yaml` using
App Store Connect relationship names and category IDs:

```yaml
primaryCategory: GAMES
primarySubcategoryOne: GAMES_ACTION
secondaryCategory: ENTERTAINMENT
```

The file supports both primary and secondary categories and their two optional
subcategories. Fields omitted from the file are left unchanged during `push`.

For App Store screenshots, `pull` writes synchronization state to
`<bundle-id>/.manifest.yaml` and names downloaded images by sequence, such as
`001.png`. During `push`, unchanged screenshots are reused by remote ID and
checksum, changed or failed uploads are replaced, remote screenshots missing
from a non-empty local display type directory are deleted, and the remaining
screenshots are reordered by local filename. After a successful push, the final
remote IDs and local checksums are written back to the app-level manifest. Use
`--dry-run` with the standalone
`fastforge appstore catalog push --app <bundle-id> --dry-run` command to review
local screenshot sets before syncing.

---

## Resource Usage

### `distribute_options.yaml`

Refer to the [Distribute Options](./distribute-options.md) page for the full configuration reference.
