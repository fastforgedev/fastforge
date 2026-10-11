# App Store Connect

English | [简体中文](../../zh-Hans/stores/appstore.md)

`fastforge appstore` calls the App Store Connect API directly, covering app queries, build uploads, version submission, review submissions, and catalogs.

Supported platforms are `IOS`, `MAC_OS`, `TV_OS`, and `VISION_OS`. Commands with a `--platform` option accept these values; `catalog pull` defaults to `IOS`.

## Authentication

```bash
export APP_STORE_CONNECT_KEY_ID=ABC123DEFG
export APP_STORE_CONNECT_ISSUER_ID=00000000-0000-0000-0000-000000000000
export APP_STORE_CONNECT_KEY_PATH="$PWD/AuthKey_ABC123DEFG.p8"
```

All three variables are required. Store API commands use API Key authentication only.

For projects managed by `direnv`, remember that non-interactive shells, CI, and agent tools do not run the interactive shell hook. Verify only whether variables are set—never print credential values—and keep every App Store command inside the same environment:

```bash
direnv status
direnv exec . fastforge appstore app view com.example.myapp
```

`direnv exec .` also discovers an `.envrc` in a parent directory. This prevents a global credential from accidentally selecting a different App Store Connect team.

## Apps

```bash
fastforge appstore app list
fastforge appstore app view com.example.myapp
```

`view` accepts a numeric App Store app ID, a bundle ID, an SKU, or an app name, tried in that order; a value that matches more than one app fails. Every `--app` option below resolves apps the same way.

“Set up a Store app” can mean three different operations:

1. Register an existing app locally in `.fastforge/config.yaml` for aggregate `fastforge store` commands.
2. Run `catalog pull` to create a local metadata snapshot of an existing remote app.
3. Create a new remote app record in App Store Connect. Neither local registration nor `catalog pull` creates one.

## Builds

```bash
fastforge appstore build upload dist/MyApp.ipa \
  --app com.example.myapp \
  --wait

fastforge appstore build upload dist/MyApp.pkg \
  --app com.example.myapp

fastforge appstore build list --app com.example.myapp --version 1.0.0
fastforge appstore build view <build-id>
fastforge appstore build wait <build-id> --timeout 30m
```

- Uploads run `xcrun altool` and therefore require macOS. A `.pkg` file is uploaded as a macOS app; any other file as an iOS app.
- `build wait` polls every 30 seconds until the processing state is `VALID`, and fails on `FAILED`, `INVALID`, or timeout. `--timeout` defaults to `30m` and accepts `m` or `s` suffixes; a bare number means seconds.
- `build upload --wait` uses a fixed 30-minute timeout and waits for the most recently uploaded build of the app. If the new build is not listed yet, it may pick an older build; when that matters, use `build list` and then `build wait <build-id>`.
- Fastforge has no TestFlight commands (beta groups, testers, beta review). Use App Store Connect or the raw API for them.

## Versions

```bash
fastforge appstore version list --app com.example.myapp
fastforge appstore version view 1.0.0 --app com.example.myapp
fastforge appstore version submit 1.0.0 \
  --app com.example.myapp \
  --build <build-id> \
  --wait
```

`version view` and `version submit` accept a version string or a numeric App Store version ID.

`version submit` associates the build, creates a review submission for the version's platform, adds the version item, and submits it for review.

- The version must already exist in App Store Connect; no Fastforge command creates versions.
- `--build latest` selects the newest build for that version string.
- The command does not check build processing, so wait for the build first.
- `--wait` uses a fixed 30-minute timeout and returns once the submission reaches `WAITING_FOR_REVIEW`, `IN_REVIEW`, `COMPLETING`, or `COMPLETE`. It does not wait for App Review approval.

## Review Submissions

```bash
fastforge appstore submission list --app com.example.myapp
fastforge appstore submission view <submission-id>
fastforge appstore submission create \
  --app com.example.myapp \
  --platform IOS
fastforge appstore submission items <submission-id>
fastforge appstore submission add-item <submission-id> \
  --item-type appStoreVersions \
  --item-id <version-id>
fastforge appstore submission remove-item <item-id>
fastforge appstore submission submit <submission-id> --wait --timeout 30m
fastforge appstore submission cancel <submission-id>
```

- `list` can filter by `--platform` and `--state`. States: `READY_FOR_REVIEW`, `WAITING_FOR_REVIEW`, `IN_REVIEW`, `UNRESOLVED_ISSUES`, `CANCELING`, `COMPLETING`, `COMPLETE`.
- `create` omits the platform when `--platform` is not given, leaving it to App Store Connect.
- `remove-item` takes a submission item ID as listed by `items`.
- `submit --wait` polls every 30 seconds until the submission reaches `WAITING_FOR_REVIEW`, `IN_REVIEW`, `COMPLETING`, or `COMPLETE`, and fails on `UNRESOLVED_ISSUES`, `CANCELING`, or timeout (`--timeout`, default `30m`).
- `add-item --item-type` accepts `appStoreVersions`, `appCustomProductPageVersions`, `appStoreVersionExperiments`, `appStoreVersionExperimentsV2`, `appEvents`, `backgroundAssetVersions`, `gameCenterAchievementVersions`, `gameCenterActivityVersions`, `gameCenterChallengeVersions`, `gameCenterLeaderboardSetVersions`, and `gameCenterLeaderboardVersions`.

## Catalog

See [Unified Catalog](catalog.md) for App Store metadata, categories, screenshots, and previews.

## Raw API

```bash
fastforge appstore api get /v1/apps --query 'filter[bundleId]=com.example.myapp'
fastforge appstore api patch /v1/appStoreVersions/<version-id> --input version.json
```

Supported methods are `get`, `post`, `patch`, and `delete`. The path must start with `/v1/`, `--query KEY=VALUE` can be repeated, and `post`/`patch` accept `--input` pointing to a JSON request body file. The response is pretty-printed as JSON.

The raw API supports App Store Connect resources without typed commands. Automation scripts should prefer existing typed commands.
