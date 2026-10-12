<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./apps/docs/public/brand-lockup.svg">
  <img alt="Fastforge" src="./apps/docs/public/brand-lockup.svg" width="480">
</picture>

# Fastforge <sub>formerly Flutter Distributor</sub>

[![pub version][pub-image]][pub-url] [![pub downloads][pub-dm-image]][pub-dm-url] [![][discord-image]][discord-url] [![melos](https://img.shields.io/badge/maintained%20with-melos-f700ff.svg?style=flat-square)](https://github.com/invertase/melos) [![All Contributors][all-contributors-image]](#contributors)

[pub-image]: https://img.shields.io/pub/v/fastforge.svg?style=flat-square
[pub-url]: https://pub.dev/packages/fastforge
[pub-dm-image]: https://img.shields.io/pub/dm/fastforge.svg
[pub-dm-url]: https://pub.dev/packages/fastforge/score
[discord-image]: https://img.shields.io/discord/884679008049037342.svg?style=flat-square
[discord-url]: https://discord.gg/zPa6EZ2jqb
[all-contributors-image]: https://img.shields.io/github/all-contributors/fastforgedev/fastforge?color=ee8449&style=flat-square

Ship every app faster to your users — Build, package, and publish with one clear configuration. Target popular distribution formats and app stores while fitting naturally into your CI/CD pipeline.

> [!WARNING]
> **Rust Migration In Progress:** The core of Fastforge is being rewritten in [Rust](https://www.rust-lang.org/) to deliver better performance, a smaller install footprint, and zero runtime dependencies on the Dart SDK. The native CLI lives in [`apps/cli/`](./apps/cli) with its core libraries in [`crates/`](./crates), and is developed in parallel with the existing Dart packages.
>
> **What this means for you:**
>
> - The current Dart-based CLI (`dart pub global activate fastforge`) continues to work and receives bug fixes.
> - The Rust CLI is released as a native binary — no Dart or Flutter SDK required to run it. See the [installation guide](docs/en/installation.md).
> - APIs and configuration formats are designed to remain compatible; any breaking changes will be clearly announced.
> - `distribute_options.yaml` and the `release` command are deprecated: they keep working for compatibility but get no new features. New configuration goes in `.fastforge/` (project) or `~/.fastforge/` (machine), and new automation in [local workflows](docs/en/workflows.md).
> - Contributions, feedback, and bug reports on the Rust implementation are very welcome — see the [Contributing](#contributing) section.

---

English | [简体中文](./README-ZH.md)

---

## Documentation

- [Native Fastforge CLI documentation](docs/en/README.md)
- [Legacy Dart CLI documentation](https://fastforge.dev/)

## Key Features

- 🚀 One-Click Build: Support for Android APK/AAB, iOS IPA, OpenHarmony HAP/APP and more
- 📦 Multi-Platform Release: Support for App Store, Google Play, Firebase, Pgyer, fir.im, etc.
- 🔄 CI/CD Integration: Perfect integration with GitHub Actions, GitLab CI, and more
- 🛠 Flexible Configuration: Support for multiple environments, flavors, and custom build arguments

### Supported Package Formats

- **Android**: [AAB](docs/en/packagers/android.md#aab), [APK](docs/en/packagers/android.md#apk)
- **iOS**: [IPA](docs/en/packagers/ios.md#ipa)
- **OpenHarmony**: [HAP](docs/en/packagers/ohos.md), [APP](docs/en/packagers/ohos.md)
- **Linux**: [AppImage](docs/en/packagers/linux.md#appimage), [DEB](docs/en/packagers/linux.md#deb), [RPM](docs/en/packagers/linux.md#rpm), [Pacman](docs/en/packagers/linux.md#pacman), [ZIP](docs/en/packagers/linux.md#zip-and-direct)
- **macOS**: [DMG](docs/en/packagers/macos.md#dmg), [PKG](docs/en/packagers/macos.md#pkg), [ZIP](docs/en/packagers/macos.md#zip)
- **Windows**: [EXE](docs/en/packagers/windows.md#exe), [MSIX](docs/en/packagers/windows.md#msix), [ZIP](docs/en/packagers/windows.md#zip-and-direct)
- **Web**: [ZIP](docs/en/packagers/web.md)
- **Any platform**: [custom](docs/en/packaging.md#custom-format) packaging commands; `direct` (the unpacked build output) on Linux, Windows and Web
- More formats coming soon...

### Supported Distribution Platforms

- [App Store](docs/en/publishers/appstore.md)
- [AppGallery](docs/en/publishers/appgallery.md)
- [Firebase](docs/en/publishers/firebase.md)
- [Firebase Hosting](docs/en/publishers/firebase.md#firebase-hosting)
- [FIR](docs/en/publishers/fir.md)
- [GitHub Releases](docs/en/publishers/github.md)
- [PGYER](docs/en/publishers/pgyer.md)
- [Play Store](docs/en/publishers/playstore.md)
- [Vercel](docs/en/publishers/vercel.md)
- [APT and Pacman repositories](docs/en/publishers/repositories.md)
- Object storage: [S3 / MinIO](docs/en/publishers/s3.md#s3--minio), [Qiniu](docs/en/publishers/s3.md#qiniu), [Alibaba Cloud OSS](docs/en/publishers/s3.md#alibaba-cloud-oss), [Tencent Cloud COS](docs/en/publishers/s3.md#tencent-cloud-cos)
- [Custom](docs/en/publishers/custom.md) shell command
- More platforms coming soon...

## Installation

The native CLI installs as a standalone binary; see the [installation guide](docs/en/installation.md). The Dart CLI installs with:

```bash
dart pub global activate fastforge
```

> **Windows users:** After activation, ensure the pub cache bin directory is in your PATH:
>
> 1. Open **System Properties** → **Advanced** → **Environment Variables**
> 2. Under **User variables**, select `Path` → **Edit**
> 3. Add `%APPDATA%\Pub\Cache\bin` and click **OK**
> 4. Restart your terminal, then try `fastforge --help`

## Quick Start

> **Note:** This quick start uses the deprecated `distribute_options.yaml` and `release` command, which still work in both CLIs. For new projects, see [Getting Started](docs/en/getting-started.md) and [local workflows](docs/en/workflows.md).

1. Add `distribute_options.yaml` to your project root:

```yaml
variables:
  PGYER_API_KEY: "your api key" # Replace with your own API keys
output: dist/
releases:
  - name: dev
    jobs:
      # Build and publish APK to PGYER
      - name: release-dev-android
        package:
          platform: android
          target: apk
          build_args:
            target-platform: android-arm,android-arm64
            dart-define:
              APP_ENV: dev
        publish_to: pgyer

      # Build and publish IPA to PGYER
      - name: release-dev-ios
        package:
          platform: ios
          target: ipa
          build_args:
            export-options-plist: ios/dev_ExportOptions.plist
            dart-define:
              APP_ENV: dev
        publish_to: pgyer
```

> **Note:** `build_args` are parameters supported by the `flutter build` command. Modify them according to your project requirements.

2. Release your app:

```bash
fastforge release --name dev
```

## CLI Commands

### Package Your App

```bash
fastforge package --platform=android --targets=aab,apk
```

### Publish a Package

```bash
fastforge publish --path dist/your-app-1.0.0+1-android.apk --targets pgyer
```

### Release (Package + Publish)

```bash
fastforge release --name dev
```

## Examples

Fastforge includes several example projects to help you get started:

- **[hello_world](https://github.com/fastforgedev/fastforge/tree/main/examples/hello_world)** - Basic example demonstrating the core functionality.
- **[multiple_flavors](https://github.com/fastforgedev/fastforge/tree/main/examples/multiple_flavors)** - Example showing how to configure multiple application flavors.
- **[custom_binary_name](https://github.com/fastforgedev/fastforge/tree/main/examples/custom_binary_name)** - Example of how to customize binary output names.

## Advanced Usage

### Environment Variables

Fastforge supports using environment variables in your configuration files. This is useful for sensitive information like API keys:

```yaml
variables:
  API_KEY: ${PGYER_API_KEY} # Uses the PGYER_API_KEY environment variable
```

### CI/CD Integration

Fastforge works well in CI/CD environments. For example, with GitHub Actions:

```yaml
jobs:
  build-and-release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
      - name: Install Fastforge
        run: dart pub global activate fastforge
      - name: Build and release
        run: fastforge release --name production
        env:
          API_KEY: ${{ secrets.API_KEY }}
```

Check the [documentation](https://fastforge.dev/) for more detailed CI/CD integration examples.

## Who's Using It?

- [Biyi](https://biyidev.com/) - A convenient translation and dictionary app.
- [Qianji](https://qianjiapp.com/) - A purely bookkeeping app.
- [Airclap](https://airclap.app/) - Send any file to any device. cross platform, ultra fast and easy to use.

## Contributing

Development and CI use Flutter 3.47.6 stable and its bundled Dart 3.13.5 SDK.
Add the Flutter SDK's `bin` directory to `PATH` and run `flutter` and `dart`
directly. The CI version is configured in [ci.yml](.github/workflows/ci.yml).
See [AGENTS.md](AGENTS.md) for repository setup and validation commands.

Contributions are welcome! If you'd like to help improve Fastforge:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please make sure to update tests as appropriate and follow the existing code style.

## Contributors

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/lijy91"><img src="https://avatars.githubusercontent.com/u/3889523?v=4?s=100" width="100px;" alt="LiJianying"/><br /><sub><b>LiJianying</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=lijy91" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://juejin.cn/user/764915820276439"><img src="https://avatars.githubusercontent.com/u/8764899?v=4?s=100" width="100px;" alt="Zero"/><br /><sub><b>Zero</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=BytesZero" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/KRTirtho"><img src="https://avatars.githubusercontent.com/u/61944859?v=4?s=100" width="100px;" alt="Kingkor Roy Tirtho"/><br /><sub><b>Kingkor Roy Tirtho</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=KRTirtho" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/laiiihz"><img src="https://avatars.githubusercontent.com/u/35956195?v=4?s=100" width="100px;" alt="LAIIIHZ"/><br /><sub><b>LAIIIHZ</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=laiiihz" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/ueki-tomohiro"><img src="https://avatars.githubusercontent.com/u/27331430?v=4?s=100" width="100px;" alt="Tomohiro Ueki"/><br /><sub><b>Tomohiro Ueki</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=ueki-tomohiro" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://cybrox.eu/"><img src="https://avatars.githubusercontent.com/u/2383736?v=4?s=100" width="100px;" alt="Sven Gehring"/><br /><sub><b>Sven Gehring</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=cybrox" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/GargantuaX"><img src="https://avatars.githubusercontent.com/u/14013111?v=4?s=100" width="100px;" alt="GargantuaX"/><br /><sub><b>GargantuaX</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=GargantuaX" title="Code">💻</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/hiperioncn"><img src="https://avatars.githubusercontent.com/u/6045710?v=4?s=100" width="100px;" alt="Hiperion"/><br /><sub><b>Hiperion</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=hiperioncn" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/GroovinChip"><img src="https://avatars.githubusercontent.com/u/4250470?v=4?s=100" width="100px;" alt="Reuben Turner"/><br /><sub><b>Reuben Turner</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=GroovinChip" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://animator.github.io"><img src="https://avatars.githubusercontent.com/u/615622?v=4?s=100" width="100px;" alt="Ankit Mahato"/><br /><sub><b>Ankit Mahato</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=animator" title="Documentation">📖</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://tienisto.com"><img src="https://avatars.githubusercontent.com/u/38380847?v=4?s=100" width="100px;" alt="Tien Do Nam"/><br /><sub><b>Tien Do Nam</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=Tienisto" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://zacksleo.top/"><img src="https://avatars.githubusercontent.com/u/3369169?v=4?s=100" width="100px;" alt="zacks"/><br /><sub><b>zacks</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=zacksleo" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/M97Chahboun"><img src="https://avatars.githubusercontent.com/u/69054810?v=4?s=100" width="100px;" alt="Mohammed  CHAHBOUN"/><br /><sub><b>Mohammed  CHAHBOUN</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=M97Chahboun" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/prateekmedia"><img src="https://avatars.githubusercontent.com/u/41370460?v=4?s=100" width="100px;" alt="Prateek Sunal"/><br /><sub><b>Prateek Sunal</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=prateekmedia" title="Code">💻</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/LailaiMaster"><img src="https://avatars.githubusercontent.com/u/19606597?v=4?s=100" width="100px;" alt="lllgm"/><br /><sub><b>lllgm</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=LailaiMaster" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://arran4.github.io/"><img src="https://avatars.githubusercontent.com/u/111667?v=4?s=100" width="100px;" alt="Arran Ubels"/><br /><sub><b>Arran Ubels</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=arran4" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://0x0.ink/"><img src="https://avatars.githubusercontent.com/u/49977991?v=4?s=100" width="100px;" alt="Sherman Chu"/><br /><sub><b>Sherman Chu</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=yeliulee" title="Code">💻</a> <a href="https://github.com/fastforgedev/fastforge/commits?author=yeliulee" title="Documentation">📖</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/Drsheppard01"><img src="https://avatars.githubusercontent.com/u/60893791?v=4?s=100" width="100px;" alt="DrSheppard"/><br /><sub><b>DrSheppard</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=Drsheppard01" title="Documentation">📖</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/cranst0n"><img src="https://avatars.githubusercontent.com/u/1173143?v=4?s=100" width="100px;" alt="cranst0n"/><br /><sub><b>cranst0n</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=cranst0n" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/duskygloom"><img src="https://avatars.githubusercontent.com/u/65943118?v=4?s=100" width="100px;" alt="duskygloom"/><br /><sub><b>duskygloom</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=duskygloom" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/imnadev"><img src="https://avatars.githubusercontent.com/u/46110906?v=4?s=100" width="100px;" alt="imnadev"/><br /><sub><b>imnadev</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=imnadev" title="Code">💻</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/jenken827"><img src="https://avatars.githubusercontent.com/u/185325381?v=4?s=100" width="100px;" alt="jenken827"/><br /><sub><b>jenken827</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=jenken827" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/kecson"><img src="https://avatars.githubusercontent.com/u/10434414?v=4?s=100" width="100px;" alt="kecson"/><br /><sub><b>kecson</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=kecson" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/zzqayy"><img src="https://avatars.githubusercontent.com/u/29256984?v=4?s=100" width="100px;" alt="zzqayy"/><br /><sub><b>zzqayy</b></sub></a><br /><a href="https://github.com/fastforgedev/fastforge/commits?author=zzqayy" title="Code">💻</a></td>
    </tr>
  </tbody>
  <tfoot>
    <tr>
      <td align="center" size="13px" colspan="7">
        <img src="https://raw.githubusercontent.com/all-contributors/all-contributors-cli/1b8533af435da9854653492b1327a23a4dbd0a10/assets/logo-small.svg">
          <a href="https://all-contributors.js.org/docs/bot/usage">Add your contributions</a>
        </img>
      </td>
    </tr>
  </tfoot>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

## License

[MIT](./LICENSE)

## Fastforge Studio

The Studio web client, local server, and shared packages live in this repository under the `studio-` package prefix. See the [Studio development guide](apps/studio-cli/README.md) for setup and checks.

## Agent Skills

Agent skills that teach Claude and other Agent-Skills-compatible harnesses how to use `fastforge` live in [`skills/`](skills/). Install them with `npx skills add fastforgedev/fastforge`; see the [skills guide](skills/README.md) for details.
