# Firebase

[English](../../en/publishers/firebase.md) | 简体中文

Fastforge 提供两个 Firebase target，均依赖系统中的 Firebase CLI。

## App Distribution

Target：`firebase`

`FIREBASE_TOKEN` 为必填。同时必须提供 Firebase 应用 ID（不是 bundle identifier）；使用 `fastforge publish` 时必须通过 `--firebase-app` 传入，因为命令会在发布前检查该选项：

```bash
export FIREBASE_TOKEN=firebase-token

fastforge publish --path dist/app.apk --target firebase \
  --firebase-app 1:1234567890:android:abcdef
```

可选参数会直接传给 `firebase appdistribution:distribute`，每个参数都可以写成 `--firebase-<name>` 选项或 `--publish-arg`：

- `release-notes`
- `release-notes-file`
- `testers`
- `testers-file`
- `groups`
- `groups-file`

```bash
fastforge publish --path dist/app.apk --target firebase \
  --firebase-app 1:1234567890:android:abcdef \
  --firebase-groups qa-team \
  --firebase-release-notes 'Internal build'
```

在 `fastforge/publish` 工作流 action 和 `fastforge release` 中，请以 `app` 参数传入应用 ID。

发布结果为 Firebase 控制台的 App Distribution 地址。

## Firebase Hosting

Target：`firebase-hosting`

`--path` 应指向需要部署的目录：

```bash
export FIREBASE_PROJECT_ID=my-project

fastforge publish --path build/web --target firebase-hosting
```

project ID 为必填。也可以用 `project-id` 参数或 `--firebase-hosting-project-id` 选项覆盖环境变量：

```bash
fastforge publish --path build/web --target firebase-hosting \
  --publish-arg project-id=my-project
```

Fastforge 会在目标目录生成 `.firebaserc` 和 `firebase.json`，随后在该目录运行 `firebase deploy`。项目自身存在 `firebase.json` 时，生成的 `firebase.json` 会沿用其 `hosting` 设置（多个站点时取第一个），例如 `headers`、`redirects`、`rewrites` 和 `cleanUrls`，并以目标目录作为站点内容：替换 `public`，在 `ignore` 中加入 `firebase.json` 和 `.firebaserc`，去掉 `target`、`predeploy` 和 `postdeploy`。设置了 `FIREBASE_TOKEN` 时会传给 CLI；已登录 Firebase CLI 时可省略，CI 中建议设置。发布结果为 CLI 输出的 `Hosting URL`。
