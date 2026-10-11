# Custom

[English](../../en/publishers/custom.md) | 简体中文

`custom` target 通过自定义 shell 命令接入没有内置发布器的服务。

## 用法

```bash
fastforge publish --path dist/app.zip --target custom \
  --publish-arg 'command=./scripts/upload.sh' \
  --publish-arg channel=stable
```

macOS / Linux 使用 `sh -c`，Windows 使用 `cmd /C`。

## 环境变量

自定义命令可读取：

- `ARTIFACT_PATH`：当前产物路径
- `PUBLISH_ARG_<KEY>`：除 `command` 外的发布参数，包括 `app-version` 以及 `fastforge publish` 发布器选项的默认值（例如 `PUBLISH_ARG_GITHUB_RELEASE_DRAFT`）

参数键会转为大写，非字母数字字符替换为下划线。例如 `release-channel` 会成为 `PUBLISH_ARG_RELEASE_CHANNEL`。

命令输出会被捕获，而不是实时显示。命令返回非零状态时发布失败，错误信息中包含其标准输出和标准错误。成功时，去除首尾空白后的标准输出会成为发布结果的 `message`（输出为空时为 `Published successfully.`）。
