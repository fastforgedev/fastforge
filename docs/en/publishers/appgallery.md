# App Gallery

Uploads OHOS app packages to Huawei AppGallery Connect.

## Set up environment variables

You need to provide [AppGallery Connect API (CN)](https://developer.huawei.com/consumer/cn/doc/app/agc-help-connect-api-0000002236015554) client id and client secret in your environment.

```bash
export APP_GALLERY_CLIENT_ID="your client id"
export APP_GALLERY_CLIENT_SECRET="your client secret"
```

## Usage

Run:

```bash
fastforge publish \
  --path dist/1.0.0+1/hello_world-1.0.0+1-ohos.app \
  --targets appgallery \
  --appgallery-app-id <app-id>
```

> The `--appgallery-app-id` argument passes the `app-id` to the publisher. You can also configure the same `app-id` in `distribute_options.yaml` (see example below).

### Configure `distribute_options.yaml`

```yaml
variables:
  APP_GALLERY_CLIENT_ID: 'your client id'
  APP_GALLERY_CLIENT_SECRET: 'your client secret'
output: dist/
releases:
  - name: dev
    jobs:
      - name: release-dev-ohos
        package:
          platform: ohos
          # AppGallery expects the 'app' package format
          target: app
          build_args:
            # optional, defaults to 'default' and corresponds to product name in OHOS
            flavor: default
        publish:
          target: appgallery
          args:
            # The App ID of your application on AppGallery Connect
            app-id: 'your app id'
```

Run:

```bash
fastforge release --name dev
```

## Related Links

- [AppGallery Connect API (CN)](https://developer.huawei.com/consumer/cn/doc/app/agc-help-connect-api-0000002236015554)
- [Create an AppGallery Connect API client (CN)](https://developer.huawei.com/consumer/cn/doc/app/agc-help-connect-api-obtain-server-auth-0000002271134661#section103mcpsimp)
- [Upload app files with AppGallery Connect API (CN)](https://developer.huawei.com/consumer/cn/doc/app/agc-help-publish-api-guide-0000002271134665#section110mcpsimp)
