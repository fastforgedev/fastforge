# fastforge <sub>原 Flutter Distributor</sub>

[![pub version][pub-image]][pub-url] [![pub downloads][pub-dm-image]][pub-dm-url] [![][discord-image]][discord-url] [![melos](https://img.shields.io/badge/maintained%20with-melos-f700ff.svg?style=flat-square)](https://github.com/invertase/melos) [![All Contributors][all-contributors-image]](#contributors)

[pub-image]: https://img.shields.io/pub/v/fastforge.svg?style=flat-square
[pub-url]: https://pub.dev/packages/fastforge
[pub-dm-image]: https://img.shields.io/pub/dm/fastforge.svg
[pub-dm-url]: https://pub.dev/packages/fastforge/score
[discord-image]: https://img.shields.io/discord/884679008049037342.svg?style=flat-square
[discord-url]: https://discord.gg/zPa6EZ2jqb
[all-contributors-image]: https://img.shields.io/github/all-contributors/fastforgedev/fastforge?color=ee8449&style=flat-square

让每一个应用更快抵达用户 — 用一套清晰的配置完成构建、打包与发布。覆盖主流分发格式和应用市场，也能自然融入团队的 CI/CD 流程。

> [!WARNING]
> **Rust 迁移进行中：** Fastforge 的核心正在用 [Rust](https://www.rust-lang.org/) 重写，以实现更好的性能、更小的安装体积，以及对 Dart SDK 零运行时依赖。新的实现位于 [`crates/`](./crates) 目录，与现有 Dart 包并行开发。
>
> **这对您意味着什么：**
> - 当前基于 Dart 的 CLI（`dart pub global activate fastforge`）仍然可用，并会持续收到问题修复。
> - Rust CLI 将以原生二进制文件发布——运行时无需安装 Dart 或 Flutter SDK。
> - API 和配置格式设计上保持兼容；任何破坏性变更都会提前明确公告。
> - 非常欢迎对 Rust 实现提交贡献、反馈和问题报告——请参阅[参与贡献](#参与贡献)部分。

---

[English](./README.md) | 简体中文

---

## 文档

- [Fastforge 原生 CLI 文档](docs/zh-Hans/README.md)
- [旧版 Dart CLI 文档](https://fastforge.dev/zh)

## 主要特性

- 🚀 一键打包：支持 Android APK/AAB、iOS IPA、OpenHarmony HAP/APP 等多种格式
- 📦 多平台发布：支持 App Store、Google Play、Firebase、蒲公英、fir.im 等
- 🔄 CI/CD 集成：完美支持 GitHub Actions、GitLab CI 等持续集成平台
- 🛠 灵活配置：支持多环境、多 flavor、自定义构建参数

### 支持的打包格式

- **Android**: [AAB](https://fastforge.dev/zh/makers/aab), [APK](https://fastforge.dev/zh/makers/apk)
- **iOS**: [IPA](https://fastforge.dev/zh/makers/ipa)
- **OpenHarmony**: [HAP](https://fastforge.dev/zh/makers/hap), [APP](https://fastforge.dev/zh/makers/app)
- **Linux**: [AppImage](https://fastforge.dev/zh/makers/appimage), [DEB](https://fastforge.dev/zh/makers/deb), [RPM](https://fastforge.dev/zh/makers/rpm), Pacman
- **macOS**: [DMG](https://fastforge.dev/zh/makers/dmg), [PKG](https://fastforge.dev/zh/makers/pkg)
- **Windows**: [EXE](https://fastforge.dev/zh/makers/exe), [MSIX](https://fastforge.dev/zh/makers/msix)
- **通用**: [ZIP](https://fastforge.dev/zh/makers/zip)
- 更多格式持续增加中...

### 支持的分发平台

- [App Store](https://fastforge.dev/zh/publishers/appstore)
- [Firebase](https://fastforge.dev/zh/publishers/firebase)
- [Firebase Hosting](https://fastforge.dev/zh/publishers/firebase-hosting)
- [FIR](https://fastforge.dev/zh/publishers/fir)
- [GitHub Releases](https://fastforge.dev/zh/publishers/github)
- [PGYER](https://fastforge.dev/zh/publishers/pgyer)
- [Play Store](https://fastforge.dev/zh/publishers/playstore)
- [Qiniu](https://fastforge.dev/zh/publishers/qiniu)
- [Vercel](https://fastforge.dev/zh/publishers/vercel)
- 更多平台持续增加中...

## 安装

```bash
dart pub global activate fastforge
```

> **Windows 用户请注意：** 激活后，请确保 pub 缓存 bin 目录已添加到 PATH 环境变量中：
> 1. 打开 **系统属性** → **高级** → **环境变量**
> 2. 在 **用户变量** 中找到 `Path`，点击 **编辑**
> 3. 添加 `%APPDATA%\Pub\Cache\bin` 并点击 **确定**
> 4. 重启终端，然后运行 `fastforge --help` 验证

## 快速开始

1. 在项目根目录添加 `distribute_options.yaml` 文件:

```yaml
variables:
  PGYER_API_KEY: "your api key" # 替换为您自己的 API 密钥
output: dist/
releases:
  - name: dev
    jobs:
      # 构建并发布 APK 到 PGYER
      - name: release-dev-android
        package:
          platform: android
          target: apk
          build_args:
            target-platform: android-arm,android-arm64
            dart-define:
              APP_ENV: dev
        publish_to: pgyer

      # 构建并发布 IPA 到 PGYER
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

> **注意:** `build_args` 是 `flutter build` 命令支持的参数，请根据您的项目需求进行修改。

2. 发布您的应用:

```bash
fastforge release --name dev
```

## CLI 命令

### 打包应用

```bash
fastforge package --platform=android --targets=aab,apk
```

### 发布应用包

```bash
fastforge publish --path dist/your-app-1.0.0+1-android.apk --targets pgyer
```

### 发布（打包 + 发布）

```bash
fastforge release --name dev
```

## 示例项目

Fastforge 包含多个示例项目，帮助您快速上手：

- **[hello_world](https://github.com/fastforgedev/fastforge/tree/main/examples/hello_world)** - 演示核心功能的基础示例。
- **[multiple_flavors](https://github.com/fastforgedev/fastforge/tree/main/examples/multiple_flavors)** - 展示如何配置多种应用风格的示例。
- **[custom_binary_name](https://github.com/fastforgedev/fastforge/tree/main/examples/custom_binary_name)** - 展示如何自定义二进制输出名称的示例。

## 高级用法

### 环境变量

Fastforge 支持在配置文件中使用环境变量，这对于 API 密钥等敏感信息非常有用：

```yaml
variables:
  API_KEY: ${PGYER_API_KEY} # 使用 PGYER_API_KEY 环境变量
```

### CI/CD 集成

Fastforge 在 CI/CD 环境中工作良好。例如，使用 GitHub Actions：

```yaml
jobs:
  build-and-release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
      - name: 安装 Fastforge
        run: dart pub global activate fastforge
      - name: 构建并发布
        run: fastforge release --name production
        env:
          API_KEY: ${{ secrets.API_KEY }}
```

更详细的 CI/CD 集成示例请查阅[文档](https://fastforge.dev/zh/)。

## 谁在使用？

- [比译](https://biyidev.com/) - 一个便捷的翻译和词典应用。
- [钱迹](https://qianjiapp.com/) - 一款纯粹记账的应用。
- [Airclap](https://airclap.app/) - 任何文件，任意设备，随意发送。简单好用的跨平台高速文件传输 APP。

## 参与贡献

开发和 CI 使用 Flutter 3.47.6 稳定版及其自带的 Dart 3.13.5 SDK。
将 Flutter SDK 的 `bin` 目录加入 `PATH`，直接运行 `flutter` 和 `dart`。
CI 版本配置见 [ci.yml](.github/workflows/ci.yml)，仓库环境与验证命令见
[AGENTS.md](AGENTS.md)。

欢迎贡献代码！如果您想帮助改进 Fastforge：

1. Fork 仓库
2. 创建您的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交您的更改 (`git commit -m '添加一些很棒的特性'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建一个 Pull Request

请确保适当更新测试并遵循现有的代码风格。

## 贡献者

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
          <a href="https://all-contributors.js.org/docs/en/bot/usage">Add your contributions</a>
        </img>
      </td>
    </tr>
  </tfoot>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

## 许可证

[MIT](./LICENSE)

## Fastforge Studio

Studio 的 Web 界面、本地服务与共享包已整合到本仓库，包名统一使用 `studio-` 前缀。开发和检查命令见 [Studio 文档](apps/studio-cli/README.md)。

## Agent Skills

用于指导 Claude 等兼容 Agent Skills 的工具使用 `fastforge` 的技能位于 [`skills/`](skills/) 目录。可通过 `npx skills add fastforgedev/fastforge` 安装，详见 [Skills 说明](skills/README.md)。
