# tar.gz

English | [简体中文](../../zh-Hans/packagers/tar-gz.md)

Create a gzip-compressed TAR archive for Linux, macOS, Windows, or Web. Compression runs inside Fastforge; no `zip`, `7z`, `tar`, or `gzip` executable is required.

```bash
fastforge package --platform linux --targets tar.gz
```

Artifacts use the normal naming template and end in `.tar.gz`, for example `dist/1.0.0/demo-1.0.0-linux.tar.gz`. Existing output is overwritten.

- Linux, Windows, and Web archives contain the build directory's contents without an extra enclosing directory.
- macOS archives contain the `.app` bundles, preserving their directory structure and symbolic links. The Rust CLI supports `--notarize` to notarize and staple the apps before archiving.
- File permissions (including executable bits), symbolic links, hidden files, and empty directories are preserved. Windows build output's PDB debug symbols are excluded by the Rust CLI, as with ZIP.

The build still needs the platform's normal SDK and host OS. Flutter projects support all four platforms; native Xcode projects support macOS and native .NET projects support Windows.

Return to the [packager overview](README.md).
