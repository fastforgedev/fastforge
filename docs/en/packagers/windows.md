# Windows

English | [简体中文](../../zh-Hans/packagers/windows.md)

Fastforge packages Flutter and native .NET Windows applications as [EXE](#exe), [MSIX](#msix), [ZIP, or a direct copy](#zip-and-direct).

## Current Status

| Build system    | `package` status                              |
| --------------- | --------------------------------------------- |
| Flutter Builder | EXE, MSIX, ZIP, and direct through CLI/action |
| .NET Builder    | EXE, MSIX, ZIP, and direct through CLI/action |

Windows packaging requires a Flutter project (`pubspec.yaml`) or a native .NET project (a `.csproj` passed through `project`; see [.NET Builder](../builders/dotnet.md)). The build runs only on a Windows host; on other hosts `fastforge package` skips a Flutter target with a warning and fails a native one. The packagers use the Flutter runner directory `build/windows/<arch>/runner/<Mode>/` (`build/windows/runner/<Mode>/` on Flutter versions before 3.15) or the `dotnet publish` directory, and artifacts are written to `dist/<version>/`.

```bash
fastforge package --targets exe,zip
```

## Configuration

Windows follows the same two-step model as Linux: stage the application directory, then add the format's native metadata. Customize it with raw files in `.fastforge/packaging/windows/`, rendered with the [packaging variables](../packaging.md#packaging-variables):

```text
.fastforge/packaging/windows/
├── shared/files/                 # installed by both EXE and MSIX
├── exe/
│   ├── setup.iss                 # one *.iss, under any name
│   └── files/                    # EXE application files
└── msix/
    ├── AppxManifest.xml          # replaces the generated manifest
    └── files/Assets/             # MSIX assets or other application files
```

The bundle is copied first, then `shared/files/`, then the format's `files/`; later files win. File and directory names and text contents are rendered, while binary files are copied unchanged. Raw metadata replaces the generated script or manifest. Several `*.iss` scripts in the EXE directory fail packaging. ZIP and direct continue to package the build output without overlays. Every format leaves out the build output's debug symbols (`*.pdb`); an overlay can still add one.

Overlays are the place for files the build does not produce, such as license notices or the Microsoft Visual C++ runtime DLLs that some plugins need on machines without the redistributable. Files placed next to the executable are installed with it:

```text
.fastforge/packaging/windows/shared/files/
├── LICENSE.txt
├── msvcp140.dll
├── vcruntime140.dll
└── vcruntime140_1.dll
```

Copy the DLLs from the `Microsoft.VC*.CRT` directory of the Visual Studio redistributable (`VC\Redist\MSVC\<version>\<arch>\`) for the architecture you build, and check its license terms before distributing them.

Common metadata comes from `project:` in `.fastforge/config.yaml`: `display_name`, `description`, `homepage`, `app_id`, and `icon`. Windows icons accept PNG or ICO. An EXE's generated script converts a project PNG to an ICO; MSIX generates appropriately sized PNG assets. Supplied MSIX assets in `files/Assets/` are kept.

The Rust CLI still reads `windows/packaging/<format>/make_config.yaml`, and MSIX also reads `pubspec.yaml`'s `msix_config` (make_config wins). Legacy fields override generated metadata defaults from `project:`; raw files take precedence over generated metadata and the legacy EXE `script_template`. New projects can use only `project:` and native files. See [`examples/hello_world`](../../../examples/hello_world/.fastforge/packaging/windows/).

## EXE

EXE creates an installer with Inno Setup. It is the only format marked as an installer, so the default artifact name ends in `-setup.exe`.

Requires Inno Setup 6. Fastforge resolves `ISCC.exe` from `INNO_SETUP_PATH`, then `C:\Program Files (x86)\Inno Setup 6`, then `iscc` on `PATH`. `INNO_SETUP_PATH` can also be set in `.fastforge/config.yaml`'s `env:`. The compiler receives the output directory and file name explicitly, so a raw script uses the configured artifact name too.

A minimal `exe/setup.iss`:

```ini
[Setup]
AppId=${APP_ID}
AppName=${APP_DISPLAY_NAME}
AppVersion=${BUILD_NAME}
DefaultDirName=${INSTALL_DIR}
OutputBaseFilename=${OUTPUT_BASE_FILENAME}

[Files]
Source: "${PACKAGING_DIRECTORY}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\${APP_DISPLAY_NAME}"; Filename: "{app}\${EXECUTABLE_NAME}"
```

Inno constants such as `{app}` and preprocessor syntax remain unchanged. The rendered script sits next to the staged directory; use `${PACKAGING_DIRECTORY}` for application files and compiler resources. `EXECUTABLE_NAME` prefers `<APP_BINARY_NAME>.exe` over bundled helper executables. Without a raw script, the generated script uses the project metadata, with ARM64 or x64 architecture settings derived from the executable's machine type (then from an `arm64` segment of the build output path). Without `setup_icon_file` or `project.icon`, its setup icon is `windows/runner/resources/app_icon.ico` when present; native .NET projects should set `project.icon`.

Legacy `make_config.yaml` keys include `app_id`, `publisher_name`, `publisher_url`, `display_name`, `executable_name`, `install_dir_name`, `setup_icon_file`, `locales`, `create_desktop_icon`, `launch_at_startup`, `privileges_required`, `architectures_allowed`, `architectures_install_in_64bit_mode`, and `script_template`, a file name in `windows/packaging/exe/`. The legacy custom script still uses Liquid syntax.

## MSIX

Requires Windows SDK `makeappx` on `PATH`, plus `signtool` when signing. The Rust CLI always packages natively, including projects that depend on the Dart `msix` package.

Place a native manifest at `msix/AppxManifest.xml`; without it, an `AppxManifest.xml` already in the bundle or a `files/` overlay is kept instead of generating one. Substituted variable values in this file are XML-escaped automatically; XML markup is preserved. Use `${APP_ID}` for the identity, `${PACKAGE_VERSION}` for the four-part version, `${PACKAGE_ARCH}` for the target architecture, `${EXECUTABLE_NAME}` for the executable, and `${MSIX_PUBLISHER}` for the publisher. The identity must meet MSIX naming rules; for example, use `dev.example.demo` without underscores.

Missing metadata is generated from legacy settings, then `project:`, then defaults. The default identity is `com.flutter.<app name without underscores>` and the default version is `a.b.c.0` (the build number is not used). The default publisher is `CN=Publisher`; set `MSIX_PUBLISHER` to match the signing certificate or Store identity. Logo assets come from `logo_path`, then `project.icon`, then the Flutter runner icon, then a placeholder. Existing assets take precedence over generation: every `Assets\*.png` the manifest references is kept when the bundle or an overlay supplies it, copied from the highest `scale-` qualified variant the bundle ships (`StoreLogo.scale-200.png`), and otherwise generated at its tile size (`Wide310x150Logo`, `Square71x71Logo`, `Square310x310Logo`, `LockScreenLogo`, or `<width>x<height>` from the name).

### Native .NET projects

For a native .NET project, the `Package.appxmanifest` next to the project file supplies the identity: `Identity Name`, `Version`, and `Publisher`, `DisplayName`, and `PublisherDisplayName` become the defaults of `APP_ID`, `PACKAGE_VERSION`, `MSIX_PUBLISHER`, and `APP_DISPLAY_NAME` (legacy `make_config.yaml` fields still override them, and `MSIX_PUBLISHER` from the environment or `env:` wins over the manifest's publisher). Without a raw `msix/AppxManifest.xml`, the project manifest itself is packaged: its capabilities, extensions, and dependencies are kept verbatim, `$targetnametoken$` and `$targetentrypoint$` are replaced by the executable and `Windows.FullTrustApplication`, the `Identity` element receives the resolved name, publisher, version, and `ProcessorArchitecture`, and the `x-generate` resource language (expanded only by MSBuild) becomes `en-us`. A raw manifest replaces it entirely and may use the variables above. A framework-dependent Windows App SDK app must declare its `Microsoft.WindowsAppRuntime` `<PackageDependency>` in the manifest, or publish with `WindowsAppSDKSelfContained=true`.

```yaml
env:
  MSIX_PUBLISHER: CN=Example Company
  MSIX_CERTIFICATE_PATH: ${MSIX_CERTIFICATE_PATH}
  MSIX_CERTIFICATE_PASSWORD: ${MSIX_CERTIFICATE_PASSWORD}
```

These variables may also be supplied directly through the process environment. Certificate settings can still be provided through legacy `certificate_path`, `certificate_password`, and `signtool_options`. Explicit packager settings take precedence, followed by the environment variables and legacy settings. `signtool_options` replaces the default `sign /fd SHA256 /a /f <certificate> /p <password>` arguments. Without signing configuration, the package is unsigned and a warning is printed. Legacy `store: true` or `sign_msix: false` skips signing.

**Compatibility change:** Rust no longer invokes `dart run msix:create` or uses its bundled test certificate. `debug` and `install_certificate` do not install certificates and print a migration warning when enabled. Install development certificates separately. `trim_logo` defaults to `true`: transparent borders are cropped before generated assets are resized; set it to `false` to preserve padding. Assets supplied by the bundle or overlays are preserved. Capabilities, file associations, protocols, aliases, and startup settings can be expressed directly in the manifest; their existing legacy keys remain supported for generated manifests. The generated manifest reads `display_name`, `publisher_display_name`, `identity_name`, `msix_version`, `architecture`, `languages`, `capabilities`, `file_extension`, `protocol_activation`, `execution_alias` (or `add_execution_alias`), `enable_at_startup`, and `publisher`. `output_path`, `output_name`, and `build_windows` are ignored; the artifact uses Fastforge's output directory and name.

Generated capabilities use the Windows schema namespaces and place device capabilities last, regardless of input order. Repeated names are removed, including the automatically added `runFullTrust`. Unknown names and devices requiring child elements (such as `usb`) require a raw `AppxManifest.xml`; they fail with an actionable error instead of being emitted as ordinary capabilities.

## ZIP and direct

`zip` archives the runner directory without debug symbols (`*.pdb`); `direct` copies it the same way to a directory named after the artifact (without an extension) in `dist/<version>/`. Both need no extra tools.

Return to the [packager overview](README.md).
