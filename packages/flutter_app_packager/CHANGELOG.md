## 0.6.5

* MakeDebConfig Add StartupWMClass support #290

## 0.6.0

* feat: Support ohos platform.

## 0.4.6

* fix(flutter_app_packager): fix build rpm on linux arm64 error (#204)
* feat: `artifact_name` template now supports `has_build_number` variable (#232)
* fix: AppImage mimetype is null (#248)
* fix: can not set icon file when packaging windows exe (#246)

## 0.4.5

* bump `shell_executor` to 0.1.6
* [FIX] Don't block rpm build if metainfo not found #202

## 0.4.4

* [Linux] Add MimeType for appimage and metainfo file support (#195)

## 0.4.2

* some fixes

## 0.3.8

* bump `archive` to 3.4.10

## 0.3.7

* Use correct architecture when making deb #150

## 0.3.6

* bump `shell_executor` to 0.1.5
* bump `msix` to 3.16.6

## 0.3.5

* [deb-maker] Supports custom binary name

## 0.3.4

* bump `shell_executor` to 0.1.4

## 0.3.2

* Update dart sdk version to ">=2.16.0 <4.0.0"
* feat: compress macOS app with 7zip

## 0.3.1

* Add `direct` maker
* [rpm] fix lib/*.so rpath before packaging (#110)
* feat:fix appdmg “icon-size” Specification。 (#113)
* Bump `shell_executor` to 0.1.2.

## 0.3.0

* **FEAT**: apk & app maker support profile mode.
* **FEAT**: Linux RPM packaging support  ([#101](https://github.com/fastforgedev/fastforge/issues/101)).

## 0.2.7

* Feat: Linux RPM packaging support (#101)

## 0.2.6

* [maker-zip] remove packagingDirectory after make

## 0.2.5

* Use `shell_executor` to execute commands
* Merge app makers into this package
* [maker-zip] without using the 7zip command

## 0.2.4

* [dmg maker] Support `code-sign` configuration item.
* [exe maker] Support use custom inno setup `script_template`. #69
* [exe maker] Support use custom installDirName. #67
* [exe maker] Add installer `setupIconFile`, `privilegesRequired` options (#79)
* Downgrade pubspec_parse to 1.1.0
* [exe maker] Set to default 64-bit mode #81

## 0.2.1

* `artifactName` adds `build_name` & `build_number` variables #66

## 0.2.0

* Fix apk, ipa makers copy wrong files #55

## 0.1.9

* [exe maker] MakeExeConfig Add `executable_name` field
* [exe maker] MakeExeConfig Add `display_name` field
* [exe maker] MakeExeConfig Add `create_desktop_icon` field
* [exe maker] MakeExeConfig Add `install_dir_name` field
* [exe maker] MakeExeConfig Add `locales` field
* [exe maker] Support chinese #57 #58

## 0.1.8

* [aab maker] support flavor arg #46

## 0.1.6

* `PowerShell` support.

## 0.1.4

* #22 zip maker supports web platform.

## 0.1.0

* First release.
