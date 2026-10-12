# .NET Builder

English | [简体中文](../../zh-Hans/builders/dotnet.md)

.NET Builder supports native Windows projects built with the .NET SDK (projects without `pubspec.yaml`): WinUI 3 / Windows App SDK, WPF, Windows Forms, and console applications. It runs only on Windows with `dotnet` on `PATH`; package from another machine with a [remote host](../remote-hosts.md).

## Current Integration

| Platform | Build result                  | Subsequent packaging                              |
| -------- | ----------------------------- | ------------------------------------------------- |
| Windows  | `dotnet publish` directory    | EXE, MSIX, ZIP, direct, and `custom` packagers    |

.NET Builder is connected to the package process in the same way as Xcode Builder. The top-level `fastforge package` command has no dedicated flag for the required `project` argument, so pass it through a local workflow's `build-args`, or from the CLI through `--flutter-build-args`, for example `fastforge package --platform windows --targets msix --flutter-build-args project=MyApp/MyApp.csproj,runtime-identifier=win-x64`. Object and array arguments such as `properties` and `extra-flags` require `build-args`.

For native projects, each target runs its own build and `flutter clean` is not run. The package format is validated before the build. In artifact names, `name` is the project file name and the version comes from the project's `Package.appxmanifest` (`Identity Version`, `a.b.c.d` → `a.b.c+d`, or `a.b.c` when `d` is `0`), else `<Version>` in the project file, else `1.0.0`; channel and flavor are not used.

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

The builder runs `dotnet publish <project> -c <configuration> [-r <rid>] [--self-contained | --no-self-contained] [-p:Platform=<platform>] [-p:<Name>=<Value> ...] -o <output> [extra flags]` and reports the publish directory as the build output. The Windows packagers stage that directory like a Flutter runner directory: `zip` and `direct` package it as-is, `exe` runs Inno Setup over it, and `msix` adapts the project's `Package.appxmanifest`; see [Windows](../packagers/windows.md).

| Argument             | Required | Description                                                                 |
| -------------------- | :------: | --------------------------------------------------------------------------- |
| `project`            |   Yes    | `.csproj`, `.fsproj`, `.vbproj`, or `.sln`                                  |
| `configuration`      |    No    | Defaults to `Release`                                                       |
| `platform`           |    No    | MSBuild `Platform` (`x64`, `ARM64`); also sets `PACKAGE_ARCH`               |
| `runtime-identifier` |    No    | `-r win-x64` / `win-arm64`                                                  |
| `self-contained`     |    No    | `true` → `--self-contained`, `false` → `--no-self-contained`                |
| `properties`         |    No    | Object of extra `-p:Name=Value` pairs                                       |
| `output`             |    No    | Publish directory; defaults to `<project dir>/bin/fastforge/<configuration>/<arch>/publish` |
| `extra-flags`        |    No    | Array of additional `dotnet publish` arguments                              |

`<arch>` in the default publish directory is `platform` lowercased, else the suffix of `runtime-identifier`, else `x64`. The packagers detect the architecture from the published executable (and from the `arm64` path segment as a fallback), so `PACKAGE_ARCH`, the MSIX `ProcessorArchitecture`, and the Inno Setup architecture settings follow the build. `EXECUTABLE_NAME` prefers `<AssemblyName>.exe`.

Notes for WinUI 3 / Windows App SDK projects:

- `platform` and `runtime-identifier` must match (`x64` with `win-x64`, `ARM64` with `win-arm64`).
- For `exe`, `zip`, and `direct`, publish an unpackaged app: set `WindowsPackageType` to `None` in the project or through `properties`.
- For `msix`, a framework-dependent Windows App SDK app needs the `Microsoft.WindowsAppRuntime` `<PackageDependency>` in its manifest, because MSBuild adds it only in its own packaging step. Alternatively set `WindowsAppSDKSelfContained` to `true` so the runtime files are published with the app.
- `dotnet publish -o` with a solution is rejected by the SDK (NETSDK1194); pass the application's `.csproj`.

A console fixture that exercises this builder lives in [`fixtures/native_windows`](../../../fixtures/native_windows/).
