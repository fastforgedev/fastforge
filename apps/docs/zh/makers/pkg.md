# pkg

将 Flutter 应用构建为 macOS PKG 安装程序包。PKG 格式是 Apple 的标准安装程序包格式，常用于分发应用、驱动程序和系统扩展。它支持安装前和安装后脚本以执行自定义安装逻辑。

> 只能在 macOS 机器上构建 PKG 目标。

## 使用方法

将 `make_config.yaml` 添加到你的项目 `macos/packaging/pkg` 目录。

你也可以将 `make_config.yaml` 添加到你的项目 `macos/packaging` 目录，以继承公共配置。

```yaml
# 必填：应用程序的安装路径前缀。
# 应用将被安装到 <install-path>/<AppName>.app。
install-path: /Applications

# 可选：用于签名的证书标识。
# 例如 "Developer ID Installer: Your Name (TEAMID)"
sign-identity: <你的签名证书>

# 可选：安装脚本目录路径。
# 支持 preinstall 和 postinstall 脚本，用于在安装过程中执行
# 自定义逻辑（如 XPC Service 注册）。
scripts: <你的脚本目录路径>
```

### 脚本目录结构

配置 `scripts` 选项后，指定的目录应包含可执行脚本。`productbuild` 会将这些脚本打包并在安装时在合适的时机执行：

```
macos/packaging/pkg/
├── make_config.yaml
└── scripts/
    ├── preinstall     # 在文件安装前执行
    └── postinstall    # 在文件安装后执行
```

脚本命名规则：
- **preinstall** — 在包文件安装前执行。
- **postinstall** — 在包文件安装后执行。

> 脚本必须具有可执行权限（`chmod +x`），安装时以 root 权限运行。

运行：

```
fastforge package --platform macos --targets pkg
```

## 配置项说明

| 配置项 | 必填 | 说明 |
|--------|------|------|
| `install-path` | 是 | 安装目录，通常为 `/Applications` |
| `sign-identity` | 否 | 用于对包签名的证书标识（如 `Developer ID Installer: ...`） |
| `scripts` | 否 | 安装脚本目录路径（详见上方说明） |

## 相关链接

- [构建和发布 macOS 应用程序](https://docs.flutter.dev/deployment/macos)
- [productbuild 手册](https://www.manpagez.com/man/1/productbuild/)
