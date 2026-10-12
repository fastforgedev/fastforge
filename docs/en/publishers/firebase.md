# Firebase

English | [简体中文](../../zh-Hans/publishers/firebase.md)

Fastforge provides two Firebase targets, both of which depend on the Firebase CLI installed on the system.

## App Distribution

Target: `firebase`

`FIREBASE_TOKEN` is required. The Firebase app ID (not the bundle identifier) is also required; with `fastforge publish` it must be passed with `--firebase-app`, because the command checks for that option before publishing:

```bash
export FIREBASE_TOKEN=firebase-token

fastforge publish --path dist/app.apk --target firebase \
  --firebase-app 1:1234567890:android:abcdef
```

Optional arguments are passed directly to `firebase appdistribution:distribute`. Each is available as a `--firebase-<name>` option or a `--publish-arg`:

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

In the `fastforge/publish` workflow action and in `fastforge release`, pass the app ID as the `app` parameter.

The publishing result is the Firebase console App Distribution URL.

## Firebase Hosting

Target: `firebase-hosting`

`--path` should point to the directory to deploy:

```bash
export FIREBASE_PROJECT_ID=my-project

fastforge publish --path build/web --target firebase-hosting
```

The project ID is required. You can override the environment variable with the `project-id` argument or the `--firebase-hosting-project-id` option:

```bash
fastforge publish --path build/web --target firebase-hosting \
  --publish-arg project-id=my-project
```

Fastforge generates `.firebaserc` and `firebase.json` in the target directory, then runs `firebase deploy` there. The generated `firebase.json` takes the `hosting` settings of the project's own `firebase.json` when there is one (the first site of several), such as `headers`, `redirects`, `rewrites`, and `cleanUrls`, and serves them from the target directory; `public` is replaced, `firebase.json` and `.firebaserc` are added to `ignore`, and `target`, `predeploy`, and `postdeploy` are dropped. `FIREBASE_TOKEN` is passed to the CLI when set; it may be omitted when the Firebase CLI is already signed in, and is recommended in CI. The publishing result is the `Hosting URL` printed by the CLI.
