# msix

将 Flutter 应用构建为 Windows MSIX 包。MSIX 是微软的现代应用打包格式，提供可靠、安全的安装体验，支持自动更新、应用权限管理和企业部署场景。

## 环境要求

- Windows 10 版本 1809 或更高版本，或 Windows 11
- [MSIX 打包 SDK](https://docs.microsoft.com/zh-cn/windows/msix/)

## 使用方法

将 `make_config.yaml` 添加到你的项目 `windows/packaging/msix` 目录。

```yaml
display_name: HelloWorld
msix_version: 1.0.0.0
# logo_path: C:\path\to\logo.png
# 目标架构：x64、x86 或 arm64。
# 如果未指定，会从构建输出目录自动检测。
# architecture: x64
```

> 查看所有配置选项：[msix](https://github.com/YehudaKremer/msix)

运行：

```
fastforge package --platform windows --targets msix
```

## 构建 ARM64 版本

在 ARM64 机器（如 Surface Pro X）上构建 ARM64 MSIX 包时，无需特殊配置——架构会自动从构建输出目录检测。

在 x64 机器（如 CI）上构建 ARM64 MSIX 包时，需要：

1. 在 `distribute_options.yaml` 中设置构建目标平台：

```yaml
build_args:
  target-platform: windows-arm64
```

2. 在 `windows/packaging/msix/make_config.yaml` 中设置 `architecture: arm64`。

> **注意：** 在 x64 机器上交叉编译 ARM64 需要安装 [ARM64 Visual Studio 构建工具](https://learn.microsoft.com/en-us/windows/arm/arm-on-windows)。

## 相关链接

- [MSIX 工具包](https://github.com/YehudaKremer/msix)
- [什么是 MSIX？](https://docs.microsoft.com/zh-cn/windows/msix/)
- [构建和发布 Windows 应用程序](https://docs.flutter.dev/deployment/windows)
