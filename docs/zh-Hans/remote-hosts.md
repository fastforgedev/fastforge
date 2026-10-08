# 远程主机

[English](../en/remote-hosts.md) | 简体中文

远程主机让 `fastforge package` 和 `fastforge run` 在另一台机器上运行，例如在 Linux 上借助 Mac 打 `ipa` 或 `dmg`，或借助 Windows 主机打 `exe`。fastforge 通过 SSH 把项目同步到主机，在那里执行同一条命令并实时输出日志，最后把产物取回本地输出目录。

> [!NOTE]
> 当前已支持 `package --host`、`run --host` 和 `host` 命令。远程 `publish` 仍在规划中。主机可以是 macOS、Linux 或 Windows（见 [Windows 主机](#windows-主机)）。

## 前提条件

- 本机有 SSH 客户端（OpenSSH），并能用密钥登录主机（脚本中执行 `ssh <host> true` 不需要额外交互）。
- 主机上安装了 fastforge，且**远程协议版本**与本机一致（带有 `fastforge host` 的近期版本均可）。可用 `fastforge host doctor` 检查。
- 主机自身的工具链：Flutter、Xcode、Android SDK、签名身份等。签名材料保留在主机上。

## 添加主机

```bash
fastforge host add mac-mini builder@192.168.1.20 --forward-env "APPLE_*,GITHUB_TOKEN"
fastforge host doctor mac-mini
```

主机按机器保存在 `~/.fastforge/hosts.yaml`（可用 `FASTFORGE_HOSTS_FILE` 覆盖），不属于任何项目：

```yaml
hosts:
  - name: mac-mini
    transport: ssh            # 默认值
    host: 192.168.1.20        # 也可以是 ~/.ssh/config 中的别名
    user: builder
    port: 22
    identity_file: ~/.ssh/id_ed25519
    workdir: ~/.fastforge/remote/workspaces   # 默认值
    fastforge: /usr/local/bin/fastforge       # 默认从远端 PATH 查找
    platforms: [android, ios, macos, web]     # 为空时由 `host doctor` 填入
    forward_env: [APPLE_*, GITHUB_TOKEN]      # 转发给远程运行的本地变量
    env:                                      # 在主机上设置并展开
      PATH: ~/fvm/default/bin:$PATH
```

远程命令在用户的登录 shell（`$SHELL -l`）中执行，因此 `.zprofile`、`.profile` 里的 `PATH` 设置会生效（用引号写成 `~/...` 的条目也会被展开）。只在 `.zshrc` 中配置的工具找不到，请用 `env` 补充。`host doctor` 会列出主机能找到的工具。

## 在主机上打包

```bash
fastforge package -p ios -t ipa --host mac-mini
```

1. 项目同步到主机上的 `<workdir>/<项目名>-<哈希>`。同步根目录是所在的 git 仓库（这样仓库内的 `path:` 依赖仍然可用），不在 git 仓库中时为当前目录。
2. 在主机上对应的目录执行 `fastforge package`，输出写到本次运行专用的目录。
3. 产物取回本地输出目录（`--output`、`distribute_options.yaml` 的 `output` 或 `dist/`），保留相对路径。

未指定 `--platform` 时，指定名称的主机会自行推断平台。路径参数（`--build-target`、`--build-export-options-plist`）必须位于同步的项目内。

中断命令（Ctrl-C）或连接断开时，远程构建会随之停止。同一项目在同一主机上同时只执行一次运行（`package` 或 `run`），其余的会等待它结束；热重载前的同步不需要等待。

### `--host auto`

`--host auto` 选择 `hosts.yaml` 中 `platforms` 包含该平台的第一台主机。未指定 `--platform` 时，平台必须能从 target 或项目结构推断出来。

如果某个平台无法在当前机器上构建，但已配置的主机可以构建，`package` 会提示可以使用 `--host auto`；它不会自动切换到远程主机。

## 在主机上运行应用

```bash
fastforge run -p macos --host mac-mini
fastforge run -p web --host mac-mini
fastforge run -p ios -d <device-id> --host mac-mini
```

`run --host` 会同步项目，并在主机上执行 `fastforge run`，连接到当前终端，`flutter run` 的按键照常可用。每次热重载（`r`）或热重启（`R`）前，fastforge 会先同步本地改动：在本地改代码，在远端重载。

运行过程中输出的回环地址（VM Service、DevTools、web 服务）会通过 `ssh -L` 转发到本机的相同端口，可以直接在本地打开。`-p web` 且未指定 `-d` 时，主机用 `web-server` 设备提供页面，在本机打开输出的 `http://localhost:<port>` 即可。

说明：

- 桌面应用显示在主机的屏幕上，主机用户需要已登录桌面会话（锁屏也可以）。在 Linux 上，如果 SSH 会话没有 `DISPLAY`/`WAYLAND_DISPLAY`，fastforge 会让应用使用该桌面会话的 Wayland 套接字（或 X 显示 `:0`）。Windows 见 [Windows 主机](#windows-主机)。连接在主机上的设备（iPhone、Android）可以用 `-d` 选择。
- 按键原样转发需要本地是 Unix 终端；其他情况（Windows 或非终端）按行发送。
- 远端终端尺寸在启动时设置，之后调整窗口大小不会同步。

## Windows 主机

Windows 主机需要开启 OpenSSH 服务端，并使用默认 shell `cmd.exe`。`fastforge host doctor` 会识别系统并保存 `os: windows`（也可以在 `host add` 时传 `--os windows`）。

- fastforge 需要在主机的 `PATH` 中（`install.ps1` 会自动添加），或者配置 `fastforge:`；其中的 `~/` 表示 `%USERPROFILE%`。
- Windows 上没有登录 shell，远程运行使用 Windows 为该用户设置的环境。`env` 中的值仍支持 `~/`、`$VAR` 和 `${VAR}`，`PATH` 用 `;` 分隔。
- `host exec` 在工作区中用 `cmd.exe` 执行命令。
- Windows 上的文件没有可执行位；账户没有创建符号链接的权限（开发者模式）时，符号链接会被跳过并给出警告。
- `run -p windows --host` 通过 SSH 构建应用，再通过交互式计划任务在已登录用户的桌面会话中启动它（通过 SSH 启动的程序没有桌面），然后用 `flutter attach` 提供热重载。窗口显示在主机的屏幕上；加上 `--remote-window` 可以在本机看到它（见[在本机显示窗口](#在本机显示窗口)）。

## 在本机显示窗口

```bash
fastforge run -p windows --host windows-laptop --remote-window
```

加上 `--remote-window` 后，应用窗口会像本地窗口一样显示在本机，可以直接点击和输入。运行输出显示应用已启动后，fastforge 会在后台把窗口显示到本机，不影响运行本身：主机把该应用的窗口（按 `windows/CMakeLists.txt` 中的 `BINARY_NAME` 匹配）移到排在它自己显示器右侧的虚拟显示器上，只传输这些窗口及其拥有的菜单、对话框。主机上的其他窗口不会出现在本机，主机自己的显示器也保持开启。运行结束时窗口随之关闭。

远程窗口基于 [dazzdesk](https://github.com/dazzlabs/dazzdesk) 实现，无需另外安装：第一次使用时，fastforge 会在构建开始前，把所需版本下载到本机和主机的 `~/.fastforge/tools/dazzdesk/<版本>/`。两台机器只接受对方的证书。

目前需要：

- Windows 主机，运行 `-p windows`；本机为 macOS 或 Windows。
- 主机上安装 [Parsec 虚拟显示驱动](https://builds.parsec.app/vdd/parsec-vdd-0.45.0.0.exe)。驱动需要知道本机屏幕的分辨率：第一次用到新分辨率时，在 Parsec VDD 设置中添加它，或者在远程窗口打开期间，在主机上以管理员身份运行一次 `%USERPROFILE%\.fastforge\tools\dazzdesk\<版本>\dazzdesk.exe host`。需要这样做时，运行输出会给出警告。
- 本机能直接访问主机的 UDP 47100 端口。窗口通过 QUIC 传输，无法用 `ssh -L` 转发，因此会连接 SSH 为该主机解析出的地址（`ssh -G`），跳板机和代理不起作用。主机的 Windows 防火墙需要放行 `%USERPROFILE%\.fastforge\tools\dazzdesk\<版本>\dazzdesk.exe`。

问题会显示在运行输出中。本机一侧的日志写入临时目录中的 `fastforge-remote-window-<pid>.log`，主机会把最近一次远程窗口的日志保存在 `%TEMP%\fastforge-remote-window.log`。

## 同步范围

同步是增量的：只发送自上次同步以来变化的文件（大小、修改时间、可执行位），本地删除的文件会在远端删除。在主机上被改动的文件会在下次同步时重新发送。

不同步的内容：

- `.gitignore`（不在 git 仓库中也生效）和 `.git/info/exclude` 匹配的文件。
- `.git`、`.dart_tool`、`.gradle`、`.fastforge/remote`（远程主机自身的状态）目录，以及本地输出目录。`.fastforge/` 的其余内容（如 `config.yaml` 和打包文件）会同步，因此 `config.yaml` 中的凭据请写成引用环境变量的 `${NAME}`，不要直接写明文。
- `.fastforgeignore` 中的规则（语法同 `.gitignore`）。

`.fastforgeignore` 可以把被忽略的文件重新纳入，例如构建需要的签名配置：

```gitignore
!android/key.properties
```

主机上的构建目录会在多次运行之间保留，增量构建和缓存（Gradle、CocoaPods、pub）可以复用。

## 在工作区中执行命令

```bash
fastforge host exec mac-mini -- flutter doctor -v
fastforge host exec mac-mini --no-sync -- ls build
```

`exec` 会先同步（除非指定 `--no-sync`），再通过登录 shell 在远端工作区执行命令。参数与 `ssh` 一样用空格拼接。`exec` 不转发环境变量。

## 安全

- 转发的变量通过 SSH 通道的 stdin 传输，不出现在命令行中，因此不会出现在主机的进程列表里。只发送 `forward_env` 中列出的变量。
- 同步的工作区会保留在主机上。请不要把密钥放进项目，或者把它们加入忽略规则。
- SSH 连接通过 `~/.fastforge/ssh` 中的套接字共享一分钟（`ControlMaster`）。
