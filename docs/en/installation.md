# Installation

English | [简体中文](../zh-Hans/installation.md)

The Fastforge CLI runs as a standalone binary. The CLI does not bundle platform SDKs, build tools, signing tools, or third-party publishing clients.

## Install with the Script

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.sh | sh
```

The default installation path is `/usr/local/bin/fastforge`. You can specify an installation directory or version (without a leading `v`). Set the variables on the `sh` side of the pipe:

```bash
curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.sh \
  | FASTFORGE_INSTALL_DIR="$HOME/.local/bin" sh

curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.sh \
  | FASTFORGE_VERSION="0.7.0" sh
```

The installation script supports:

- macOS: Apple Silicon and Intel
- Linux: AArch64 and x86_64 (GNU libc)

If the target directory is not writable, the script requests `sudo` access. Set `GITHUB_TOKEN` to avoid GitHub API rate limits when the script looks up the latest release.

### Windows

Run the following in PowerShell:

```powershell
iwr https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.ps1 | iex
```

The script supports x86_64 and ARM64. The default installation path is `%LOCALAPPDATA%\fastforge\bin`, which is added to the current user's `PATH`. You can specify a version or directory with environment variables:

```powershell
$env:FASTFORGE_VERSION = "0.7.0"
$env:FASTFORGE_INSTALL_DIR = "$HOME\bin"
iwr https://raw.githubusercontent.com/fastforgedev/fastforge/main/install.ps1 | iex
```

## Install from Source

Installing from source requires the latest stable Cargo toolchain:

```bash
git clone https://github.com/fastforgedev/fastforge.git
cd fastforge
cargo install --path apps/cli
```

During development, you can run the CLI directly from the repository root without installing it:

```bash
cargo run -p fastforge_cli -- --help
```

## Verify the Installation

```bash
fastforge --version
fastforge --help
```

If your shell cannot find the command, confirm that the installation directory is in `PATH`.

Binaries built from source print an `UNOFFICIAL BUILD` notice on stderr at startup; official release binaries do not.

## Upgrade

```bash
fastforge upgrade
fastforge upgrade --force   # reinstall the latest release even if already up to date
```

`upgrade` downloads the latest release archive for the current platform and replaces the running binary in place. If the binary is in a directory you cannot write to (for example `/usr/local/bin`), re-run it with `sudo` (or an elevated terminal on Windows), or re-run the installation script. To install a specific version, re-run the installation script with `FASTFORGE_VERSION`.

By default every command first checks GitHub releases for a newer version and prints a hint to stderr (5-second timeout, never fails the command). Pass `--no-version-check` to skip it, for example in CI. `fastforge version-check` runs the check on demand; `fastforge version-check --current-only` prints the local version without network access.

## Project Toolchains

Install the dependencies required by the operations you use:

| Scenario                  | Additional dependencies                                                 |
| ------------------------- | ----------------------------------------------------------------------- |
| Android build or analysis | Android SDK; APK analysis requires `aapt2` under `ANDROID_HOME` (or `ANDROID_SDK_ROOT`) |
| AAB analysis              | Android SDK `aapt2`, or bundletool specified through `BUNDLETOOL`       |
| iOS / macOS builds        | macOS, Xcode, and command-line tools                                    |
| Native Windows builds     | Windows and the .NET SDK (`dotnet` in `PATH`); the Windows App SDK for WinUI 3 |
| Flutter builds            | Flutter SDK and the target platform toolchain, with `flutter` in `PATH` |
| App Store uploads         | macOS, `xcrun`, and valid App Store Connect credentials                 |
| Firebase publishing       | Firebase CLI                                                            |
| Vercel publishing         | Vercel CLI                                                              |

## Uninstall

macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/fastforgedev/fastforge/main/uninstall.sh \
  | sh
```

Windows:

```powershell
iwr https://raw.githubusercontent.com/fastforgedev/fastforge/main/uninstall.ps1 | iex
```

If you used `FASTFORGE_INSTALL_DIR` during installation, pass the same value when uninstalling.
