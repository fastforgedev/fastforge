# tar.gz

Package Flutter apps for Linux, macOS, Windows, or Web as gzip-compressed TAR archives. No external compression tools are required.

```bash
fastforge package --platform linux --targets tar.gz
```

The output is written to `dist/<version>/<artifact>.tar.gz`. Linux, Windows, and Web archives contain the build directory's contents; macOS archives contain the `.app` bundles. File permissions, symbolic links, hidden files, and empty directories are preserved. Re-running the command overwrites the archive.

No `make_config.yaml` is needed. Build requirements remain the same as for other formats on the chosen platform.
