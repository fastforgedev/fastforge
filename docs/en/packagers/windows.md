# Windows

English | [简体中文](../../zh-Hans/packagers/windows.md)

Fastforge packages Flutter Windows applications as [EXE](#exe), [MSIX](#msix), [ZIP, or a direct copy](#zip-and-direct).

## Current Status

| Build system    | `package` status                              |
| --------------- | --------------------------------------------- |
| Flutter Builder | EXE, MSIX, ZIP, and direct through CLI/action |

Windows packaging requires a Flutter project (`pubspec.yaml`). The build runs only on a Windows host; on other hosts `fastforge package` skips the target with a warning. The packagers use the runner directory `build/windows/<arch>/runner/<Mode>/` (`build/windows/runner/<Mode>/` on Flutter versions before 3.15), and artifacts are written to `dist/<version>/`.

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

The bundle is copied first, then `shared/files/`, then the format's `files/`; later files win. File and directory names and text contents are rendered, while binary files are copied unchanged. Raw metadata replaces the generated script or manifest. Several `*.iss` scripts in the EXE directory fail packaging. ZIP and direct continue to package the build output as-is.

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

Inno constants such as `{app}` and preprocessor syntax remain unchanged. The rendered script sits next to the staged directory; use `${PACKAGING_DIRECTORY}` for application files and compiler resources. `EXECUTABLE_NAME` prefers `<APP_BINARY_NAME>.exe` over bundled helper executables. Without a raw script, the generated script uses the project metadata, with ARM64 or x64 architecture settings derived from the build output.

Legacy `make_config.yaml` keys include `app_id`, `publisher_name`, `publisher_url`, `display_name`, `executable_name`, `install_dir_name`, `setup_icon_file`, `locales`, `create_desktop_icon`, `launch_at_startup`, `privileges_required`, and `script_template`. The legacy custom script still uses Liquid syntax.

## MSIX

Requires Windows SDK `makeappx` on `PATH`, plus `signtool` when signing. The Rust CLI always packages natively, including projects that depend on the Dart `msix` package.

Place a native manifest at `msix/AppxManifest.xml`. Substituted variable values in this file are XML-escaped automatically; XML markup is preserved. Use `${APP_ID}` for the identity, `${PACKAGE_VERSION}` for the four-part version, `${PACKAGE_ARCH}` for the target architecture, `${EXECUTABLE_NAME}` for the executable, and `${MSIX_PUBLISHER}` for the publisher. The identity must meet MSIX naming rules; for example, use `dev.example.demo` without underscores.

Missing metadata is generated from legacy settings, then `project:`, then defaults. The default identity is `com.flutter.<app name without underscores>` and the default version is `a.b.c.0` (the build number is not used). The default publisher is `CN=Publisher`; set `MSIX_PUBLISHER` to match the signing certificate or Store identity. Logo assets come from `logo_path`, then `project.icon`, then the Flutter runner icon, then a placeholder. Existing assets take precedence over generation.

```yaml
env:
  MSIX_PUBLISHER: CN=Example Company
  MSIX_CERTIFICATE_PATH: ${MSIX_CERTIFICATE_PATH}
  MSIX_CERTIFICATE_PASSWORD: ${MSIX_CERTIFICATE_PASSWORD}
```

These variables may also be supplied directly through the process environment. Certificate settings can still be provided through legacy `certificate_path`, `certificate_password`, and `signtool_options`. Explicit packager settings take precedence, followed by the environment variables and legacy settings. Without signing configuration, the package is unsigned and a warning is printed. Legacy `store: true` or `sign_msix: false` skips signing.

**Compatibility change:** Rust no longer invokes `dart run msix:create` or uses its bundled test certificate. `debug` and `install_certificate` do not install certificates; `trim_logo` is not applied. Install development certificates separately. Capabilities, file associations, protocols, aliases, and startup settings can be expressed directly in the manifest; their existing legacy keys remain supported for generated manifests.

## ZIP and direct

`zip` archives the runner directory; `direct` copies it to `dist/<version>/` without an archive. Both need no extra tools.

Return to the [packager overview](README.md).
