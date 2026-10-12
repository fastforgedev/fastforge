# iOS

[English](../../en/packagers/ios.md) | 简体中文

Fastforge 构建并整理 iOS 应用产物，输出格式为 [IPA](#ipa)。构建 iOS 产物需要 macOS 与 Xcode。

## 当前状态

| 构建系统        | 适用项目            | `package` 状态                        |
| --------------- | ------------------- | ------------------------------------- |
| Xcode           | 不含 `pubspec.yaml` | IPA 已支持；推荐使用 package action   |
| Flutter Builder | 含 `pubspec.yaml`   | IPA 已通过 CLI 和 package action 支持 |

除未签名的 Flutter IPA 外，两条路径都需要导出配置。Xcode Builder 还需要 `project` 和 `scheme`，推荐通过工作流 package action 的 `build-args` 传入。打包后的 IPA 会复制到 `dist/<version>/`。

## IPA

IPA 是 iOS 应用的归档分发格式。

Flutter 项目需要提供 export options plist：

```bash
fastforge package --targets ipa \
  --build-export-options-plist ios/ExportOptions.plist
```

也可以通过 `--flutter-build-args export-method=app-store` 指定导出方式。完整参数见 [Flutter Builder](../builders/flutter.md)。

如需不签名打包（例如稍后再签名，或用于 CI 检查），使用 `--flutter-build-args no-codesign`。此时无需导出配置：Flutter 在生成 Xcode archive 后停止，Fastforge 把其中的 `.app` 以 `Payload/<name>.app` 的形式打包为 IPA。

### 签名

签名由 Xcode 在导出 archive 时完成，因此在 export options plist 和 Xcode 项目中配置，而不是在 Fastforge 中。自动签名只需 `method` 和 `teamID`；手动签名还需为每个 Bundle ID 指定描述文件：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>method</key>
  <string>app-store-connect</string>
  <key>teamID</key>
  <string>ABCDE12345</string>
  <key>signingStyle</key>
  <string>manual</string>
  <key>provisioningProfiles</key>
  <dict>
    <key>com.example.app</key>
    <string>Example App Store Profile</string>
  </dict>
</dict>
</plist>
```

`method` 可为 `app-store-connect`、`release-testing`（ad hoc）、`enterprise` 或 `debugging`（Xcode 15.3 之前为 `app-store`、`ad-hoc`、`development`）。签名证书和描述文件需安装在构建机上；在 CI 中，请在运行 Fastforge 前把证书导入临时钥匙串，并把描述文件复制到 `~/Library/MobileDevice/Provisioning Profiles/`。

Xcode 项目通过工作流打包：

```yaml
- name: Package IPA
  uses: fastforge/package
  with:
    platform: ios
    target: ipa
    output: artifacts/
    build-args: '{"project":"ios/MyApp.xcodeproj","scheme":"MyApp","export-options-plist":"ios/ExportOptions.plist"}'
```

完整参数见 [Xcode Builder](../builders/xcode.md#ios)。

Flutter 项目只需要原始 IPA 时：

```bash
fastforge build \
  --platform ios \
  --target ipa \
  --build-export-method app-store
```

### 发布到 App Store

```bash
fastforge publish --path dist/<version>/<artifact>.ipa --targets appstore
```

凭证、上传和审核流程见 [App Store 发布器](../publishers/appstore.md)与 [App Store Connect](../stores/appstore.md)。

返回[打包器总览](README.md)。
