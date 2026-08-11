# pkg

Build your Flutter app as a macOS PKG installer package. The PKG format is Apple's standard installer package format, commonly used for distributing applications, drivers, and system extensions. It supports pre-install and post-install scripts for custom installation logic.

> You can only build the PKG target on macOS machines.

## Usage

Add `make_config.yaml` to your project `macos/packaging/pkg` directory.

You can also add `make_config.yaml` to your project `macos/packaging` directory to inherit common configuration.

```yaml
# Required: Installation path prefix for the app bundle.
# The app will be installed as <install-path>/<AppName>.app.
install-path: /Applications

# Optional: The signing identity for product signing.
# e.g. "Developer ID Installer: Your Name (TEAMID)"
sign-identity: <your-sign-identity>

# Optional: Path to a directory containing installation scripts.
# Supports preinstall and postinstall scripts for custom logic
# during package installation (e.g., XPC service registration).
scripts: <your-scripts-path>
```

### Scripts Directory Structure

When you configure the `scripts` option, the specified directory should contain executable scripts. `productbuild` will include them in the package and run them at the appropriate time during installation:

```
macos/packaging/pkg/
├── make_config.yaml
└── scripts/
    ├── preinstall     # Runs before file installation
    └── postinstall    # Runs after file installation
```

Script naming conventions:
- **preinstall** — Executed before the package files are installed.
- **postinstall** — Executed after the package files are installed.

> Scripts must be executable (`chmod +x`). They run as root during installation.

Run:

```
fastforge package --platform macos --targets pkg
```

## Configuration Options

| Option | Required | Description |
|--------|----------|-------------|
| `install-path` | Yes | Installation directory, usually `/Applications` |
| `sign-identity` | No | Certificate identity for signing the package (e.g. `Developer ID Installer: ...`) |
| `scripts` | No | Path to installation scripts directory (see above) |

## Related Links

- [Build and release a macOS app](https://docs.flutter.dev/deployment/macos)
- [productbuild man page](https://www.manpagez.com/man/1/productbuild/)
