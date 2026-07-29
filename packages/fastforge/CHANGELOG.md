## 0.6.12

* refactor: move the distributor implementation into `fastforge` and remove
  the `unified_distributor` package

## 0.6.11

* fix(flutter): use supported build version flags (#354)
* bump `unified_distributor` to ^0.2.11

## 0.6.10

* fix(rpm): enforce usage of `appBinaryName` for assets and fix symlink logic (#313)
* fix(rpm): rename `packagerEmail` to `packager_email` in JSON config deserialization (#352)
* bump `unified_distributor` to ^0.2.10

## 0.6.9

* fix(msix): auto-detect architecture from build output directory (#349)
* fix(linux-pacman): include 'opt/' directory in bsdtar archive (#346)
* feat: add configurable EXE installer architectures
* fix: desktop entry version key (#347)

## 0.6.8

* fix: ensure non-zero exit code on packaging/publishing errors (#344)
* fix(macos): edit pkg PackageInfo in-place and handle productsign errors
* fix(qiniu): clamp sent value within bounds to prevent progress bar overflow
* fix: improved error handling to catch both Error and Exception types on publishing failures
* bump `unified_distributor` to ^0.2.8
* bump `qiniu_sdk_base` to ^0.8.0

## 0.6.7

* feat: add custom packager support (#326)
* feat: add prepackage/postpackage hooks support (#342)
* refactor: install Linux app files under `/opt` instead of `/usr/share` (#327)
* feat(rpm): add support for RPM spec macros configuration
* fix(appimage): skip running ldd on directories
* feat: support custom Inno Setup installation path via `INNO_SETUP_PATH` env var
* feat(Inno Setup): support extra environment variables and locale filtering
* fix(macos): use `--component` flag for productbuild and add scripts support
* bump `unified_distributor` to ^0.2.7

## 0.6.6

* feat: add comprehensive pgyer upload parameters support #297

## 0.6.5

* MakeDebConfig Add StartupWMClass support #290
* feat: Add AppGallery publisher support

## 0.6.4

* fix: The `savekey-prefix` argument in the minio publisher is now properly ignored when empty

## 0.6.3

* feat: Support `minio` publisher

## 0.6.2

* feat: Support `app-version` argument to override version from pubspec.yaml (future versions will not read from pubspec.yaml)

## 0.6.1

* GitHub publisher supports `repo` argument to replace `repo-owner` and `repo-name` arguments
* GitHub publisher supports `release-draft` and `release-prerelease` arguments
* Remove AppCenter publisher

## 0.6.0

* [FIX] Google Play Bundle is uploaded but the result is ignored (#261)
* Better error if entity is not a file otherwise it looks like this: (#266)
* feat: Support ohos platform.

## 0.5.1

* Bump `unified_distributor` to 0.1.1

## 0.5.0

* Keeping version in sync with flutter_distributor

## 0.1.0

* First release.
