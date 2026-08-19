# Huawei AppGallery Connect

English | [简体中文](../../zh-Hans/stores/appgallery.md)

`fastforge appgallery` calls the AppGallery Connect Publishing API directly. The generated client covers app lookup and metadata, package queries, upload preparation, and release submission.

## Authentication

Service accounts are recommended by Huawei. Set `APP_GALLERY_SERVICE_ACCOUNT_JSON` to either the downloaded credential JSON or its file path:

```bash
export APP_GALLERY_SERVICE_ACCOUNT_JSON=/secure/appgallery-private.json
```

`APP_GALLERY_SERVICE_ACCOUNT_KEY` is accepted as a path-only alias.

The legacy API client flow is also supported:

```bash
export APP_GALLERY_CLIENT_ID=client-id
export APP_GALLERY_CLIENT_SECRET=client-secret
```

When both are present, service-account authentication takes precedence.

## Apps

```bash
fastforge appgallery app resolve com.example.myapp
fastforge appgallery app view <app-id>
fastforge appgallery app view <app-id> --lang en-US --json appInfo,languages
```

## Packages

```bash
fastforge appgallery package list <app-id>
fastforge appgallery package status <app-id> <package-id>
```

## Submit for Review

```bash
fastforge appgallery release <app-id>
fastforge appgallery release <app-id> --release-time "2026-08-20T08:00:00+0800"
```

## Raw API

Endpoints not yet exposed as typed commands can be called with an authenticated relative path:

```bash
fastforge appgallery api get /api/publish/v2/app-info \
  --query appId=<app-id>
fastforge appgallery api put /api/publish/v2/app-language-info \
  --query appId=<app-id> --input language.json
```

The checked-in source specification is [`scripts/generate/specs/app_gallery_connect.openapi.yaml`](../../../scripts/generate/specs/app_gallery_connect.openapi.yaml). Huawei publishes HTML API reference pages rather than a downloadable OAS document, so this normalized OpenAPI 3 file is maintained from the official reference and regenerated with:

```bash
python3 scripts/generate/app_gallery_connect.py
```

Official references: [Getting Started](https://developer.huawei.com/consumer/en/doc/appgallery-connect-guides/agcapi-getstarted-0000001111845114), [Querying App Information](https://developer.huawei.com/consumer/en/doc/AppGallery-connect-References/agcapi-app-info-query-0000001158365045).
