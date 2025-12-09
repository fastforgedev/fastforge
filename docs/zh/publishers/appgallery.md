# App Gallery

将 OHOS App 包上传到华为 AppGallery Connect。

## 设置环境变量

需要在环境中提供 [AppGallery Connect API](https://developer.huawei.com/consumer/cn/doc/app/agc-help-connect-api-0000002236015554) 的 client id 和 client secret

```bash
export APP_GALLERY_CLIENT_ID="your client id"
export APP_GALLERY_CLIENT_SECRET="your client secret"
```

## 用法

运行：

```bash
fastforge publish \
  --path dist/1.0.0+1/hello_world-1.0.0+1-ohos.app \
  --targets appgallery \
  --appgallery-app-id <app-id>
```

> `--appgallery-app-id` 将 `app-id` 参数传递给发布器。你也可以在 `distribute_options.yaml` 中配置相同的 `app-id`（见下例）。

### 配置 `distribute_options.yaml`

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
          # 应用市场仅支持 'app' 格式
          target: app
          build_args:
            # 可选，默认为 default, 对应 ohos 中的 product name
            flavor: default
        publish:
          target: appgallery
          args:
            # 应用在 AppGallery Connect 上的 APP ID
            app-id: 'your app id'
```

运行：

```bash
fastforge release --name dev
```

## 相关链接

- [AppGallery Connect API](https://developer.huawei.com/consumer/cn/doc/app/agc-help-connect-api-0000002236015554)
- [创建 AppGallery Connect API 客户端](https://developer.huawei.com/consumer/cn/doc/app/agc-help-connect-api-obtain-server-auth-0000002271134661#section103mcpsimp)
- [通过 AppGallery Connect API 上传应用文件](https://developer.huawei.com/consumer/cn/doc/app/agc-help-publish-api-guide-0000002271134665#section110mcpsimp)
