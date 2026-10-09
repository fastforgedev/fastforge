# fastforge_app_packager

A unified application packager — packages Flutter build outputs into platform-specific distribution formats.

## Status

Work in progress (WIP). Supports packaging for Android, iOS, macOS, Linux, Windows, Web, and OpenHarmony.

## Supported Formats

| Platform | Format | Packager |
|---|---|---|
| Android | APK | `AndroidApkPackager` |
| Android | AAB | `AndroidAabPackager` |
| iOS | IPA | `IOSIpaPackager` |
| macOS | DMG | `MacOSDmgPackager` |
| macOS | PKG | `MacOSPkgPackager` |
| macOS | ZIP | `MacOSZipPackager` |
| Linux | AppImage | `LinuxAppImagePackager` |
| Linux | DEB | `LinuxDebPackager` |
| Linux | RPM | `LinuxRpmPackager` |
| Linux | Pacman | `LinuxPacmanPackager` |
| Linux | ZIP | `LinuxZipPackager` |
| Linux | Direct | `LinuxDirectPackager` |
| Windows | EXE | `WindowsExePackager` |
| Windows | MSIX | `WindowsMsixPackager` |
| Windows | ZIP | `WindowsZipPackager` |
| Windows | Direct | `WindowsDirectPackager` |
| Web | ZIP | `WebZipPackager` |
| Web | Direct | `WebDirectPackager` |
| OpenHarmony | HAP | `OHOSHapPackager` |
| OpenHarmony | APP | `OHOSAppPackager` |

## Native desktop packaging

Linux and Windows stage application files before adding format metadata. Raw
files and shared/format-specific `files/` overlays live in
`.fastforge/packaging/<platform>/`. Windows EXE accepts one `exe/*.iss` script;
MSIX accepts `msix/AppxManifest.xml` and always uses the Windows SDK. Existing
`make_config.yaml` remains readable for compatibility. See the
[Windows packaging guide](../../docs/en/packagers/windows.md).

## API Usage

```rust
use fastforge_app_packager::AndroidApkPackager;
use fastforge_core::{AppPackager, PackageConfig};

fn package_apk() -> anyhow::Result<()> {
    let packager = AndroidApkPackager;
    let config = PackageConfig::new("build/app/outputs/flutter-apk")?;
    let result = packager.package(&config)?;

    println!("Package created at: {}", result.output_path);
    Ok(())
}
```

## CLI Usage

```bash
cargo run -p fastforge_app_packager -- <platform> <format> <input-dir> <output-dir>
```

## Run Tests

```bash
cargo test -p fastforge_app_packager --offline
```

`tests/windows_packagers.rs` uses executable stand-ins on Unix to check tool
arguments, environment, staging, and output routing. It does not validate a
Windows installer; that requires Inno Setup and the Windows SDK on Windows.

On Windows, `tests/windows_native.rs` compiles the generated and raw EXE/MSIX
configurations with real tools, installs/runs/uninstalls the generated EXE in
a temporary directory, and unpacks MSIX to inspect the installed files:

```powershell
cargo test -p fastforge_app_packager --test windows_native -- --ignored --nocapture
```

Set `MSIX_CERTIFICATE_PATH`, `MSIX_CERTIFICATE_PASSWORD`, and `MSIX_PUBLISHER`
to also exercise signing. The test does not install or trust the certificate.
Set `FASTFORGE_NATIVE_ARTIFACTS` to retain packages in a chosen directory.
