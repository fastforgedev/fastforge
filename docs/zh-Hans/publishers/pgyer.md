# 蒲公英

[English](../../en/publishers/pgyer.md) | 简体中文

`pgyer` target 上传应用安装包到蒲公英（PGYER）。

## 配置

```bash
export PGYER_API_KEY=pgyer-api-key
```

## 发布

```bash
fastforge publish --path dist/app.apk --target pgyer
```

文件扩展名（例如 `apk`、`ipa`）会作为蒲公英的构建类型提交，请保证扩展名准确。上传后 Fastforge 每 3 秒查询一次构建信息，蒲公英仍在处理时最多重试 10 次。发布结果为 `http://www.pgyer.com/<build key>`。

## 可选参数

每个参数都可以写成 `--pgyer-<name>` 选项或 `--publish-arg`：

| 参数                 | 说明                                  |
| -------------------- | ------------------------------------- |
| `oversea`            | 上传加速：`1` 海外，`2` 国内          |
| `install-type`       | `1` 公开，`2` 密码，`3` 邀请          |
| `password`           | 安装密码（密码安装时使用）            |
| `description`        | 应用描述                              |
| `update-description` | 本次版本更新描述                      |
| `install-date`       | 安装有效期：`1` 设置时间段，`2` 永久  |
| `install-start-date` | 有效期开始日期，例如 `2018-01-01`     |
| `install-end-date`   | 有效期结束日期，例如 `2018-12-31`     |
| `channel-shortcut`   | 要更新的渠道短链接                    |

数值型参数如果传入非数字，会被忽略。

```bash
fastforge publish --path dist/app.apk --target pgyer \
  --pgyer-install-type 2 \
  --pgyer-password 1234 \
  --pgyer-update-description 'Internal build'
```
