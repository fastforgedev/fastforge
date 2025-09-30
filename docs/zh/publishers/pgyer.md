# Pgyer

The pgyer target publishes your `.apk` or `.ipa` artifacts to the [pgyer.com](https://pgyer.com).

## 设置环境变量

需要设置一些环境变量才能正确运行。

```
export PGYER_API_KEY="your api key"
```

## 用法

运行：

```
fastforge publish \
  --path dist/1.0.0+1/hello_world-1.0.0+1-android.apk \
  --targets pgyer
```

## 可选参数

你可以提供额外的参数来自定义上传：

- `--pgyer-oversea`: 上传加速选择（1=海外加速，2=国内加速，空=自动判断）
- `--pgyer-install-type`: 应用安装方式（1=公开安装，2=密码安装，3=邀请安装，默认：1）
- `--pgyer-password`: 应用安装密码（密码安装时必需）
- `--pgyer-description`: 应用介绍（可选）
- `--pgyer-update-description`: 版本更新描述（可选）
- `--pgyer-install-date`: 是否设置安装有效期（1=设置有效时间，2=长期有效，可选）
- `--pgyer-install-start-date`: 安装有效期开始时间（如：2018-01-01）
- `--pgyer-install-end-date`: 安装有效期结束时间（如：2018-12-31）
- `--pgyer-channel-shortcut`: 所需更新指定的渠道短链接（如：abcd）
