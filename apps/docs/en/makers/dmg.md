# dmg

Build your Flutter app as a macOS DMG (Apple Disk Image) package for distribution. DMG files are the standard format for distributing macOS applications, providing a compressed disk image that users mount to install the app by dragging it to the Applications folder.

> You can only build the DMG target on macOS machines.

## Requirements

- macOS
- `appdmg` — a Node.js tool for creating macOS disk images

  Run the following command:

  ```
  npm install -g appdmg
  ```

## Usage

Add `make_config.yaml` to your project `macos/packaging/dmg` directory.

You can also add `make_config.yaml` to your project `macos/packaging` directory to inherit common configuration.

```yaml
title: hello_world
contents:
  - x: 448
    y: 344
    type: link
    path: '/Applications'
  - x: 192
    y: 344
    type: file
    path: hello_world.app
```

Run:

```
fastforge package --platform macos --targets dmg
```

## Related Links

- [node-appdmg](https://github.com/LinusU/node-appdmg)
- [Build and release a macOS app](https://docs.flutter.dev/deployment/macos)
