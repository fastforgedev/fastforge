# dmg

将 Flutter 应用构建为 macOS DMG（Apple 磁盘映像）包进行分发。DMG 文件是分发 macOS 应用的标准格式，它提供一个压缩的磁盘映像，用户挂载后可通过拖拽应用到 Applications 文件夹完成安装。

> 只能在 macOS 机器上构建 DMG 目标。

## 环境要求

- macOS 操作系统
- `appdmg` — 用于创建 macOS 磁盘映像的 Node.js 工具

  运行以下命令安装：

  ```
  npm install -g appdmg
  ```

## 使用方法

将 `make_config.yaml` 添加到你的项目 `macos/packaging/dmg` 目录。

你也可以将 `make_config.yaml` 添加到你的项目 `macos/packaging` 目录，以继承公共配置。

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

运行：

```
fastforge package --platform macos --targets dmg
```

## 相关链接

- [node-appdmg](https://github.com/LinusU/node-appdmg)
- [构建和发布 macOS 应用程序](https://docs.flutter.dev/deployment/macos)
