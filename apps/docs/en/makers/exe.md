# exe

Build your Flutter app as a Windows EXE installer using Inno Setup. This creates a traditional Windows setup executable that guides users through the installation process, including creating desktop shortcuts and Start menu entries.

## Requirements

- Windows system
- [`Inno Setup 6`](https://jrsoftware.org/isinfo.php) — a free installer builder for Windows programs

## Usage

Add `make_config.yaml` to your project `windows/packaging/exe` directory.

```yaml
# The value of AppId uniquely identifies this application.
# Do not use the same AppId value in installers for other applications.
app_id: 5B599538-42B1-4826-A479-AF079F21A65D
publisher: LeanFlutter
publisher_url: https://github.com/leanflutter/fastforge
display_name: Hello 世界
create_desktop_icon: true
# See: https://jrsoftware.org/ishelp/index.php?topic=setup_defaultdirname
# install_dir_name: "D:\\HELLO-WORLD"
# This path is relative to the root directory of your project; The format of icon file must be ico, can not be png or others
# setup_icon_file: windows\runner\resources\app_icon.ico
locales:
  - en
  - zh
# Space-separated list of architecture identifiers the installer is allowed to run on.
# See: https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesallowed
# Defaults to `x64compatible` if not specified.
# architectures_allowed: x64compatible

# Space-separated list of architectures that should enable 64-bit install mode.
# See: https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesinstallin64bitmode
# Defaults to `x64compatible` if not specified.
# architectures_install_in_64bit_mode: x64compatible
```

Run:

```
fastforge package --platform windows --targets exe
```

## Advanced Usage

### Custom Inno Setup Installation Path

By default, `fastforge` looks for Inno Setup at the default installation path (`C:\Program Files (x86)\Inno Setup 6`). If you installed Inno Setup in a custom location (e.g., via [Scoop](https://scoop.sh) or a portable version), you can specify the path using the `INNO_SETUP_PATH` environment variable.

```bash
# PowerShell
$env:INNO_SETUP_PATH = "D:\Tools\Inno Setup 6"
fastforge package --platform windows --targets exe

# CMD
set INNO_SETUP_PATH=D:\Tools\Inno Setup 6
fastforge package --platform windows --targets exe
```

If `INNO_SETUP_PATH` is not set, `fastforge` will check the default path first, then fall back to looking for `iscc` in your system `PATH` (which is useful when Inno Setup is installed via Scoop or added to PATH manually).

### Target Architecture

By default, the generated installer allows installation on **x64-compatible systems** (both native x64 and ARM64 with x64 emulation). You can customize this behavior using the `architectures_allowed` and `architectures_install_in_64bit_mode` options.

```yaml
# Allow installation on both x64 and ARM64 systems
architectures_allowed: x64compatible
architectures_install_in_64bit_mode: x64compatible
```

- `architectures_allowed` — Specifies which CPU architectures the installer is allowed to run on. Values are space-separated architecture identifiers or boolean expressions. See the [Inno Setup documentation](https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesallowed) for available values.
- `architectures_install_in_64bit_mode` — Specifies which architectures should enable 64-bit install mode (e.g., `{autopf64}` for default install directory). Values are space-separated architecture identifiers or boolean expressions. See the [Inno Setup documentation](https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesinstallin64bitmode) for available values.

Common architecture identifiers: `x86`, `x64`, `arm64`, `x64compatible`, `x86compatible`. Note that `x64compatible` matches both native x64 systems and ARM64 systems running x64 emulation (e.g., Windows 11 on ARM), while `x64` only matches native x64 hardware.

If not specified, both options default to `x64compatible`.

### Custom Inno Setup Template

By default, `fastforge` will generate an Inno Setup configuration (`.iss`) based on an internal template on build time, and populate it with the values provided in `make_config.yaml`. If you need more control over the Inno Setup configuration, you can provide a custom template using the `script_template` option.

For example:

1. Add `script_template: inno_setup.iss` to your `make_config.yaml`
2. Create the `inno_setup.iss` in the same directory
3. Copy the [original template](https://github.com/leanflutter/fastforge/blob/main/packages/flutter_app_packager/lib/src/makers/exe/inno_setup/inno_setup_script.dart) from the source code and adjust it.

## Related Links

- [Inno Setup](https://jrsoftware.org/isinfo.php)
- [Inno Setup Documentation](https://jrsoftware.org/ishelp/)
