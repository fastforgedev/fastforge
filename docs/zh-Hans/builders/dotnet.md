# .NET Builder

[English](../../en/builders/dotnet.md) | 简体中文

.NET Builder 用于使用 .NET SDK 构建的原生 Windows 项目（不含 `pubspec.yaml`）：WinUI 3 / Windows App SDK、WPF、Windows Forms 和控制台应用。它只能在 `PATH` 中有 `dotnet` 的 Windows 上运行；在其他机器上可通过[远程主机](../remote-hosts.md)打包。

## 当前接入范围

| 平台    | 构建结果               | 后续打包                                   |
| ------- | ---------------------- | ------------------------------------------ |
| Windows | `dotnet publish` 目录  | EXE、MSIX、ZIP、direct 和 `custom` 打包器 |

.NET Builder 与 Xcode Builder 以相同方式接入 package 流程。顶层 `fastforge package` 没有必填参数 `project` 的专用选项，可通过本地工作流的 `build-args` 传入，也可以通过 CLI 的 `--flutter-build-args` 传入，例如 `fastforge package --platform windows --targets msix --flutter-build-args project=MyApp/MyApp.csproj,runtime-identifier=win-x64`；`properties`、`extra-flags` 等对象和数组参数需要使用 `build-args`。

对于原生项目，每个 target 单独构建，不执行 `flutter clean`，打包格式在构建前校验。产物名中的 `name` 取项目文件名，版本取项目的 `Package.appxmanifest`（`Identity Version`，`a.b.c.d` 转为 `a.b.c+d`，`d` 为 `0` 时为 `a.b.c`），其次取项目文件中的 `<Version>`，否则为 `1.0.0`；不使用 channel 和 flavor。

## Windows

```yaml
- name: Package Windows app
  uses: fastforge/package
  with:
    platform: windows
    target: msix
    output: artifacts/
    build-args: |
      {
        "project": "apps/desktop/windows/MyApp/MyApp.csproj",
        "configuration": "Release",
        "platform": "x64",
        "runtime-identifier": "win-x64",
        "self-contained": true,
        "properties": {"RustConfiguration": "release"},
        "extra-flags": ["--nologo"]
      }
```

构建器执行 `dotnet publish <project> -c <configuration> [-r <rid>] [--self-contained | --no-self-contained] [-p:Platform=<platform>] [-p:<Name>=<Value> ...] -o <output> [extra flags]`，并把发布目录作为构建输出。Windows 打包器像处理 Flutter runner 目录一样暂存该目录：`zip` 和 `direct` 直接打包，`exe` 用 Inno Setup 生成安装程序，`msix` 则改写项目的 `Package.appxmanifest`，详见 [Windows](../packagers/windows.md)。

| 参数                 | 必填 | 说明                                                                        |
| -------------------- | :--: | --------------------------------------------------------------------------- |
| `project`            |  是  | `.csproj`、`.fsproj`、`.vbproj` 或 `.sln`                                   |
| `configuration`      |  否  | 默认 `Release`                                                              |
| `platform`           |  否  | MSBuild `Platform`（`x64`、`ARM64`），同时决定 `PACKAGE_ARCH`               |
| `runtime-identifier` |  否  | `-r win-x64` / `win-arm64`                                                  |
| `self-contained`     |  否  | `true` 为 `--self-contained`，`false` 为 `--no-self-contained`              |
| `properties`         |  否  | 额外 `-p:Name=Value` 键值对象                                               |
| `output`             |  否  | 发布目录，默认 `<项目目录>/bin/fastforge/<configuration>/<arch>/publish`    |
| `extra-flags`        |  否  | 额外 `dotnet publish` 参数数组                                              |

默认发布目录中的 `<arch>` 取小写的 `platform`，其次取 `runtime-identifier` 的后缀，否则为 `x64`。打包器根据发布的可执行文件判断架构（找不到时回退到路径中的 `arm64` 段），因此 `PACKAGE_ARCH`、MSIX 的 `ProcessorArchitecture` 和 Inno Setup 的架构设置都与构建一致。`EXECUTABLE_NAME` 优先使用 `<AssemblyName>.exe`。

WinUI 3 / Windows App SDK 项目注意事项：

- `platform` 与 `runtime-identifier` 必须匹配（`x64` 对应 `win-x64`，`ARM64` 对应 `win-arm64`）。
- `exe`、`zip` 和 `direct` 需要发布未打包的应用：在项目中或通过 `properties` 把 `WindowsPackageType` 设为 `None`。
- `msix` 打包依赖框架的 Windows App SDK 应用时，清单中需要包含 `Microsoft.WindowsAppRuntime` 的 `<PackageDependency>`，因为 MSBuild 只在自身的打包步骤中添加它；也可以把 `WindowsAppSDKSelfContained` 设为 `true`，让运行时文件随应用一起发布。
- SDK 不允许对解决方案使用 `dotnet publish -o`（NETSDK1194），请传入应用的 `.csproj`。

[`fixtures/native_windows`](../../../fixtures/native_windows/) 中有一个用于测试该构建器的控制台项目。
