# msix

Build your Flutter app as a Windows MSIX package. MSIX is Microsoft's modern application packaging format that provides a reliable, secure installation experience with support for automatic updates, app permissions management, and enterprise deployment scenarios.

## Requirements

- Windows 10 version 1809 or later, or Windows 11
- [MSIX Packaging SDK](https://docs.microsoft.com/en-us/windows/msix/)

## Usage

Add `make_config.yaml` to your project `windows/packaging/msix` directory.

```yaml
display_name: HelloWorld
msix_version: 1.0.0.0
# logo_path: C:\path\to\logo.png
# Target architecture: x64, x86, or arm64.
# If not specified, auto-detected from the build output directory.
# architecture: x64
```

> View all configuration options: [msix](https://github.com/YehudaKremer/msix)

Run:

```
fastforge package --platform windows --targets msix
```

## Build for ARM64

To build an ARM64 MSIX package on an ARM64 machine (e.g., Surface Pro X), no special configuration is needed — the architecture is auto-detected from the build output directory.

To build an ARM64 MSIX package on an x64 machine (e.g., CI), you need to:

1. Set the build target platform in `distribute_options.yaml`:

```yaml
build_args:
  target-platform: windows-arm64
```

2. Set `architecture: arm64` in your `windows/packaging/msix/make_config.yaml`.

> **Note:** Cross-compiling for ARM64 on an x64 machine requires the [ARM64 Visual Studio build tools](https://learn.microsoft.com/en-us/windows/arm/arm-on-windows) to be installed.

## Related Links

- [MSIX Toolkit](https://github.com/YehudaKremer/msix)
- [What is MSIX?](https://docs.microsoft.com/en-us/windows/msix/)
- [Build and release a Windows app](https://docs.flutter.dev/deployment/windows)
