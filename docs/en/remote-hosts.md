# Remote Hosts

English | [简体中文](../zh-Hans/remote-hosts.md)

Remote hosts let `fastforge package` and `fastforge run` run on another machine, for example building an `ipa` or `dmg` on a Mac from Linux, or an `exe` on a Windows box. fastforge syncs the project to the host over SSH, runs the same command there, streams its output, and fetches the artifacts into your local output directory.

> [!NOTE]
> Currently supported: `package --host`, `run --host` and the `host` command. Remote `publish` is planned. Hosts can run macOS, Linux or Windows (see [Windows hosts](#windows-hosts)).

## Requirements

- An SSH client on the local machine (OpenSSH), and key-based login to the host (`ssh <host> true` must work without extra prompts in scripts).
- fastforge installed on the host, with the **same remote protocol version** (any recent release that has `fastforge host`). `fastforge host doctor` checks it.
- The host's own toolchain: Flutter, Xcode, the Android SDK, signing identities, and so on. Signing material stays on the host.

## Adding a host

```bash
fastforge host add mac-mini builder@192.168.1.20 --forward-env "APPLE_*,GITHUB_TOKEN"
fastforge host doctor mac-mini
```

Hosts are stored per machine in `~/.fastforge/hosts.yaml` (override with `FASTFORGE_HOSTS_FILE`) and are never part of a project:

```yaml
hosts:
  - name: mac-mini
    transport: ssh            # default
    host: 192.168.1.20        # or an alias from ~/.ssh/config
    user: builder
    port: 22
    identity_file: ~/.ssh/id_ed25519
    workdir: ~/.fastforge/remote/workspaces   # default
    fastforge: /usr/local/bin/fastforge       # default: found on the remote PATH
    platforms: [android, ios, macos, web]     # filled in by `host doctor` when empty
    forward_env: [APPLE_*, GITHUB_TOKEN]      # local variables sent to remote runs
    env:                                      # set on the host, expanded there
      PATH: ~/fvm/default/bin:$PATH
```

Remote commands run in the user's login shell (`$SHELL -l`), so `PATH` entries from `.zprofile`/`.profile` apply (entries written as a quoted `~/...` are expanded too). Tools set up only in `.zshrc` are not found; add them with `env`. `host doctor` lists which tools the host can see.

## Packaging on a host

```bash
fastforge package -p ios -t ipa --host mac-mini
```

1. The project is synced to `<workdir>/<project>-<hash>` on the host. The synced root is the enclosing git repository (so `path:` dependencies inside it keep working), or the current directory outside one.
2. `fastforge package` runs in the matching directory on the host, with its output in a per-run directory.
3. The artifacts are fetched into the local output directory (`--output`, `output` from `distribute_options.yaml`, or `dist/`), keeping their relative paths.

Without `--platform`, a named host infers the platform itself. Path options (`--build-target`, `--build-export-options-plist`) must point inside the synced project.

Stopping the command (Ctrl-C) or losing the connection stops the remote build. Only one run (`package` or `run`) per project and host executes at a time; others wait for it. Syncing before a hot reload doesn't wait.

### `--host auto`

`--host auto` picks the first host in `hosts.yaml` whose `platforms` contains the platform. Without `--platform`, the platform must follow from the targets or the project layout.

When a platform can't be built on the current machine but a configured host can build it, `package` prints a hint to use `--host auto`; it never switches to a remote host by itself.

## Running the app on a host

```bash
fastforge run -p macos --host mac-mini
fastforge run -p web --host mac-mini
fastforge run -p ios -d <device-id> --host mac-mini
```

`run --host` syncs the project and runs `fastforge run` on the host, attached to your terminal: the `flutter run` keys work as usual. Before each hot reload (`r`) or hot restart (`R`), fastforge syncs your local changes, so you edit locally and reload remotely.

Loopback URLs the run prints (VM service, DevTools, the web server) are forwarded to the same ports on this machine with `ssh -L`, so they open locally. For `-p web` without `-d`, the host serves the app with the `web-server` device; open the printed `http://localhost:<port>` here.

Notes:

- Desktop apps open on the host's screen, so the host user must be logged in to a desktop session (a locked screen is fine). On Linux, fastforge points the app at that session's Wayland socket (or X display `:0`) when the SSH session has no `DISPLAY`/`WAYLAND_DISPLAY`. On Windows, see [Windows hosts](#windows-hosts). Devices plugged into the host (iPhone, Android) can be selected with `-d`.
- Raw keyboard relay needs a local Unix terminal. Elsewhere (Windows, or without a terminal) keys are sent line by line.
- The remote terminal size is set when the run starts; resizing later isn't relayed.

## Windows hosts

Windows hosts need the OpenSSH server with its default shell, `cmd.exe`. `fastforge host doctor` detects the OS and saves `os: windows` (or pass `--os windows` to `host add`).

- fastforge must be on the host's `PATH` (`install.ps1` adds it), or set `fastforge:`; `~/` there means `%USERPROFILE%`.
- There is no login shell: remote runs see the user's environment as Windows sets it up. Values in `env` still accept `~/`, `$VAR` and `${VAR}`; separate `PATH` entries with `;`.
- `host exec` runs the command with `cmd.exe` in the workspace.
- Files keep no executable bit on Windows, and symbolic links are skipped with a warning unless the account may create them (Developer Mode).
- `run -p windows --host` builds and starts the app with hot reload and a forwarded VM service, but the window can't appear: programs started over SSH have no desktop. Use `-p web` (served and forwarded to this machine) for UI work, or run on the Windows machine itself.

## What gets synced

Syncs are incremental: only files that changed (size, modification time, executable bit) since the last sync are sent, and files deleted locally are removed remotely. Files changed on the host are sent again on the next sync.

Excluded:

- Everything matched by `.gitignore` files (also outside git repositories) and `.git/info/exclude`.
- The directories `.git`, `.dart_tool`, `.gradle`, `.fastforge/remote` (the remote host's own state), and the local output directory. The rest of `.fastforge/`, such as `config.yaml` and the packaging files, is synced, so keep credentials in `config.yaml` as `${NAME}` references to environment variables rather than literal values.
- Patterns in `.fastforgeignore` (same syntax as `.gitignore`).

`.fastforgeignore` can re-include ignored files, for example a signing configuration the build needs:

```gitignore
!android/key.properties
```

Build directories on the host are kept between runs, so incremental builds and caches (Gradle, CocoaPods, pub) are reused.

## Running commands in the workspace

```bash
fastforge host exec mac-mini -- flutter doctor -v
fastforge host exec mac-mini --no-sync -- ls build
```

`exec` syncs first (unless `--no-sync`), then runs the command in the remote workspace through the login shell. The arguments are joined with spaces, like `ssh`. Variables are not forwarded.

## Security

- Forwarded variables are sent through the SSH channel's stdin, not the command line, so they don't show in the host's process list. Only variables listed in `forward_env` are sent.
- The synced workspace persists on the host. Keep secrets out of the project or ignore them.
- SSH connections are shared for a minute (`ControlMaster`) through sockets in `~/.fastforge/ssh`.
