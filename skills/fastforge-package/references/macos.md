# macOS packaging (DMG / PKG / ZIP)

Requires macOS with Xcode command-line tools. Fastforge builds the `.app`
(Xcode Builder or Flutter Builder, chosen from project files) and hands it to
the selected packager.

## Flutter projects — direct CLI

```bash
fastforge package --targets dmg,pkg,zip   # --platform macos inferred; one build, three artifacts
```

Non-Android platforms build once and reuse the `.app` across all requested
targets. Flavors, dart-defines, and custom entry points use the same
`--build-*` options listed in [android.md](android.md); artifacts are written
to `dist/<version>/`. The `.app` is looked up in
`build/macos/Build/Products/<Mode>[-<flavor>]/`, preferring `PRODUCT_NAME`
from `macos/Runner/Configs/AppInfo.xcconfig`.

## Native Xcode projects (no `pubspec.yaml`)

Recommended: pass the project through `build-args` in a workflow:

```yaml
- name: Package macOS app
  uses: fastforge/package
  with:
    platform: macos
    target: zip
    output: artifacts/
    build-args: '{"project":"MyApp.xcodeproj","scheme":"MyApp","configuration":"Release","derived-data-path":"build"}'
```

String arguments also work from the CLI via
`--flutter-build-args project=MyApp.xcodeproj,scheme=MyApp,derived-data-path=build`
(`extra-flags` needs the action).

| `build-args` field | Required | Meaning |
| --- | :-: | --- |
| `project` | yes | Path to `.xcodeproj` (passed as `-project`) |
| `scheme` | yes | Xcode scheme |
| `configuration` | no | Defaults to `Release` |
| `derived-data-path` | no | Passed as `-derivedDataPath`; the `.app` is then searched in `<path>/Build/Products/<configuration>/` |
| `product-name` | no | `.app` name to match (otherwise the first `*.app`) |
| `sdk` | no | Passed to `xcodebuild -sdk` |
| `xcconfig-override` | no | Extra xcconfig file |
| `extra-flags` | no | Array of extra xcodebuild arguments |

Without `derived-data-path`, fastforge searches
`<directory of project>/build/<configuration>/` for the `.app`; set
`derived-data-path` so the products location is explicit.

Native caveats: the target is validated only after the Xcode build, each
target rebuilds, name/version come from the built app's `Info.plist`, and
channel/flavor are not used in the artifact name.

## Format notes

- **DMG** — uses the built-in DMG maker (no `appdmg` Node tool needed). It
  copies `macos/packaging/dmg/` next to the `.app` and uses an appdmg-style
  spec from `macos/packaging/dmg/make_config.yaml` when present (title, icon,
  background, window, contents…; assets resolved relative to that directory),
  otherwise a default layout (app + `/Applications` link, `background.png` if
  present).
- **PKG** — built with `xcrun productbuild --component` (plus `pkgutil`
  fixes). Reads optional `macos/packaging/pkg/make_config.yaml` with keys
  `install-path` (default `/Applications/`), `sign-identity` (runs
  `productsign`), `scripts`; a missing file means defaults, an unparsable
  one fails. A PKG can be uploaded to the Mac App Store:
  `fastforge publish --path dist/<version>/MyApp-….pkg --targets appstore`.
- **ZIP** — copies the `.app` and compresses it with **`7z`** (must be on
  `PATH`); ideal for GitHub Releases, S3, or a download server via
  fastforge-publish.

## Signing and notarization

fastforge does not sign the app: the `.app` from the build must already be
signed with a "Developer ID Application" identity, hardened runtime, and a
secure timestamp (PKG `sign-identity` only signs the installer). Built-in
notarization is opt-in: `--notarize` on the CLI (`notarize: "true"` on the
action) or `notarize: true` in `macos/packaging/<dmg|pkg|zip>/make_config.yaml`.
It notarizes and staples the `.app` before zipping or imaging it, signs the
DMG with `--timestamp`, and notarizes and staples the DMG and PKG (PKG needs a
"Developer ID Installer" `sign-identity`). Credentials come from the
environment: `NOTARYTOOL_KEYCHAIN_PROFILE`, or `APP_STORE_CONNECT_KEY_ID` +
`APP_STORE_CONNECT_KEY_PATH` (+ `APP_STORE_CONNECT_ISSUER_ID`), or `APPLE_ID`
+ `APPLE_APP_SPECIFIC_PASSWORD` + `APPLE_TEAM_ID`. Other custom steps still go
through the packaging hooks (`--hook-post` / `hook-post`). Verify
the result afterwards with `fastforge analyze <path/to/app.dmg>`, which
reports signing type, notarization stapling, and Gatekeeper state.
