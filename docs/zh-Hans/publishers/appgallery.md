# AppGallery

[English](../../en/publishers/appgallery.md) | 简体中文

`appgallery` target 上传应用包到 Huawei AppGallery Connect。

## 认证

```bash
export APP_GALLERY_CLIENT_ID=client-id
export APP_GALLERY_CLIENT_SECRET=client-secret
```

## 发布

`app-id` 为必填发布参数，也可以通过 `--appgallery-app-id` 选项传入：

```bash
fastforge publish --path dist/app.aab --target appgallery \
  --publish-arg app-id=appgallery-app-id
```

也可以用 `client-id` 和 `client-secret` 发布参数覆盖环境变量，但不建议把秘密写入命令历史。

发布器会依次获取 access token、申请上传地址、上传文件并提交包信息。发布结果为 AppGallery Connect 控制台地址。

该 target 只支持上述 API 客户端凭证；服务账号变量（`APP_GALLERY_SERVICE_ACCOUNT_JSON`）只由 `fastforge appgallery` 读取。上传应用包不会提交审核发布，请使用 `fastforge appgallery`，见 [AppGallery Connect](../stores/appgallery.md)。
