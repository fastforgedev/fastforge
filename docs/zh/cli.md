# CLI

如何使用 Fastforge 的命令行界面（CLI）

## 安装

```shell
dart pub global activate fastforge
```

## 命令

> 这些命令按字母顺序排列，最常用的是 package、publish 和 release。

### Package

将应用程序打包为特定于平台的格式，并将结果放入文件夹中。

<table><thead><tr><th>Flag</th><th>Value</th><th data-type="checkbox">Required</th></tr></thead><tbody><tr><td><code>--platform</code></td><td>平台, 如 <code>android</code></td><td>true</td></tr><tr><td><code>--targets</code></td><td>以逗号分隔的 maker 名称列表</td><td>true</td></tr><tr><td><code>--skip-clean</code></td><td>跳过构建前的清理</td><td>false</td></tr></tbody></table>

示例：

```shell
fastforge package --platform=android --targets=aab,apk
```

### Publish

<table><thead><tr><th>Flag</th><th>Value</th><th data-type="checkbox">Required</th></tr></thead><tbody><tr><td><code>--path</code></td><td>路径, 如 <code>hello_world-1.0.0+1-android.apk</code></td><td>true</td></tr><tr><td><code>--targets</code></td><td>以逗号分隔的 publisher 名称列表</td><td>true</td></tr></tbody></table>

示例：

```shell
fastforge publish --path hello_world-1.0.0+1-android.apk --targets fir,pgyer
```

### Release

会根据配置文件（`distribute_options.yaml`），将你的应用打包成特定的格式并发布到分发平台。

<table><thead><tr><th>Flag</th><th>Value</th><th data-type="checkbox">Required</th></tr></thead><tbody><tr><td><code>--name</code></td><td>名称, e.g. <code>dev</code></td><td>true</td></tr><tr><td><code>--skip-clean</code></td><td>跳过构建前的清理</td><td>false</td></tr></tbody></table>

示例：

```shell
fastforge release --name dev
```
