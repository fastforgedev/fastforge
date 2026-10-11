# App Store

English | [简体中文](../../zh-Hans/publishers/appstore.md)

The `appstore` target uses macOS `xcrun altool` to upload an IPA or PKG to App Store Connect.

## Requirements

- macOS and Xcode command-line tools
- A correctly signed `.ipa` or `.pkg` (`.ipa` is uploaded as `ios`; other files as `osx`)
- One of the following authentication methods

## API Key Authentication

```bash
export APP_STORE_CONNECT_KEY_ID=ABC123DEFG
export APP_STORE_CONNECT_ISSUER_ID=00000000-0000-0000-0000-000000000000
export APP_STORE_CONNECT_KEY_PATH="$PWD/AuthKey_ABC123DEFG.p8"
```

The key ID and issuer ID are required together. Compatible variables `APPSTORE_APIKEY` and `APPSTORE_APIISSUER` are read first.

`APP_STORE_CONNECT_KEY_PATH` is optional. Without it, `altool` looks for `AuthKey_<key id>.p8` in its default `private_keys` folders. With it, Fastforge copies the key into a temporary `private_keys` folder for the upload.

## Username Authentication

```bash
export APPSTORE_USERNAME=user@example.com
export APPSTORE_PASSWORD=app-specific-password
```

The username and app-specific password are required together.

## Upload

```bash
fastforge publish --path dist/MyApp.ipa --target appstore
```

The publisher also accepts the `key-id` (or `api-key`), `issuer-id` (or `api-issuer`), `key-path`, `username`, and `password` arguments. Avoid passing sensitive credentials this way, since they end up in command history. The publishing result is `https://appstoreconnect.apple.com/apps`.

## Subsequent Management

Completing an upload does not submit the app for review. Use `fastforge appstore` to query builds, associate versions, and create review submissions; see [App Store Connect](../stores/appstore.md).
