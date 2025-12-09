## 0.6.6

* feat: add comprehensive pgyer upload parameters support #297

## 0.6.5

* feat: Add AppGallery publisher support

## 0.6.4

* fix: The `savekey-prefix` argument in the minio publisher is now properly ignored when empty

## 0.6.3

* feat: Support `minio` publisher

## 0.6.2

* Publisher now supports `app-version` argument to override version from pubspec.yaml (future versions will not read from pubspec.yaml)

## 0.6.1

* GitHub publisher supports `repo` argument to replace `repo-owner` and `repo-name` arguments
* GitHub publisher supports `release-draft` and `release-prerelease` arguments

## 0.6.0

* Better error if entity is not a file otherwise it looks like this: (#266)

## 0.4.6

* feat: Use GitHub Actions environment variables as fallback when custom variables are not defined (#217)
* fix: Resolve GooglePlay publishing error (status: 400, message: "This edit has already been committed") (#214)

## 0.4.5

* bump `shell_executor` to 0.1.6
* bump `parse_app_package` to 0.4.5

## 0.4.4

* Support set track for playstore deployment (#185)

## 0.4.2

* some fixes

## 0.4.1

* [playstore] - Replace `GOOGLE_APPLICATION_CREDENTIALS` to `PLAYSTORE_CREDENTIALS`

## 0.4.0

* bump `parse_app_package` to 0.4.0

## 0.3.6

* bump `shell_executor` to 0.1.5
* bump `dio` to 5.3.4
* bump `googleapis` to 9.1.0

## 0.3.4

* bump `shell_executor` to 0.1.4

## 0.3.2

* Update dart sdk version to ">=2.16.0 <4.0.0"

## 0.3.1

* Add `firebase-hosting` publisher.
* Add `vercel` publisher.
* Modify the `publish` method to accept `FileSystemEntity` instead of just `File`

## 0.3.0

* Update a dependency to the latest release.

## 0.2.5

* Use `shell_executor` to execute commands
* Merge app publishers into this package
* [publisher-pgyer] Upgrade to v2 Api #91 #92

## 0.2.3

* Downgrade pubspec_parse to 1.1.0

## 0.2.0

* Add `appcenter` publisher. #13

## 0.1.8

* Add `appstore` publisher. #45

## 0.1.6

* Fix the problem of broken files after uploading.

## 0.1.5

* Add `firebase` publisher.
* Add `github` publisher.

## 0.1.4

* Add `qiniu` publisher.

## 0.1.0

* First release.
