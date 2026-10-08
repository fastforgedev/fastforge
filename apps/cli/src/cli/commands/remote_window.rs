//! `fastforge run --host <name> --remote-window`: the app's window on this
//! machine.
//!
//! Once the app runs, the host starts `dazzdesk host`
//! (<https://github.com/dazzlabs/dazzdesk>) in its desktop session, moving
//! the app's window onto a virtual display, and this machine shows that
//! window with `dazzdesk client`. Users only see "remote window"; dazzdesk
//! is an implementation detail. Each side trusts only the other's
//! certificate fingerprint. Only Windows hosts are supported for now.

use anyhow::{Context, Result, anyhow, bail};
use fastforge_remote::interactive::notice;
use fastforge_remote::{HostConfig, RemoteClient, RemoteOs};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::run_desktop;
use super::upgrade::{download, extract};
use super::version_check::{ReleaseAsset, http_client, release_target};

/// The dazzdesk release `--remote-window` runs, downloaded on first use on
/// each machine.
const DAZZDESK_VERSION: &str = "0.2.0";
const DAZZDESK_REPO: &str = "dazzlabs/dazzdesk";

/// Runs this dazzdesk binary instead of the downloaded one (development).
const DAZZDESK_ENV: &str = "FASTFORGE_DAZZDESK";

fn dazzdesk_file_name() -> &'static str {
    if cfg!(windows) {
        "dazzdesk.exe"
    } else {
        "dazzdesk"
    }
}

/// `~/.fastforge/tools/dazzdesk/<version>/dazzdesk`.
fn managed_dazzdesk() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot find the home directory"))?;
    Ok(home
        .join(".fastforge")
        .join("tools")
        .join("dazzdesk")
        .join(DAZZDESK_VERSION)
        .join(dazzdesk_file_name()))
}

/// Where the pinned release's archive for `target` is published.
fn dazzdesk_archive(target: &str) -> (String, String) {
    let ext = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let name = format!("dazzdesk-{DAZZDESK_VERSION}-{target}.{ext}");
    let url =
        format!("https://github.com/{DAZZDESK_REPO}/releases/download/v{DAZZDESK_VERSION}/{name}");
    (name, url)
}

/// The dazzdesk binary behind remote windows, downloading the pinned
/// release on first use.
pub async fn ensure_dazzdesk() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(DAZZDESK_ENV).filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let dest = managed_dazzdesk()?;
    if dest.is_file() {
        return Ok(dest);
    }
    let target = release_target()?;
    let (name, url) = dazzdesk_archive(target);
    let dir = dest.parent().expect("versioned directory");
    std::fs::create_dir_all(dir).with_context(|| format!("Failed to create {}", dir.display()))?;
    // Unpacked beside the destination, so moving it into place is atomic.
    let work = dir.join(format!(".download-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    eprintln!("Setting up remote windows (first use)...");
    let result = async {
        let client = http_client(Some(Duration::from_secs(600)))?;
        let archive = work.join(&name);
        let asset = ReleaseAsset {
            name: name.clone(),
            browser_download_url: url.clone(),
            size: 0,
        };
        download(&client, &asset, &archive).await?;
        extract(&archive, &work)?;
        let binary = work
            .join(format!("dazzdesk-{DAZZDESK_VERSION}-{target}"))
            .join(dazzdesk_file_name());
        if !binary.is_file() {
            bail!("{} has no {}", name, dazzdesk_file_name());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&binary, &dest)?;
        anyhow::Ok(())
    }
    .await;
    let _ = std::fs::remove_dir_all(&work);
    result.with_context(|| format!("Could not set up remote windows from {url}"))?;
    Ok(dest)
}

/// How long the host's `dazzdesk host` gets to start listening.
const HOST_START_TIMEOUT: Duration = Duration::from_secs(30);

/// How long `dazzdesk client` gets to connect before the run goes ahead
/// without waiting for it.
const CLIENT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// The line `remote-agent dazzdesk-host` prints once the Host listens.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostReady {
    pub fingerprint: String,
    pub port: u16,
}

/// The fingerprint and port `dazzdesk host` prints once it listens.
pub fn parse_host_output(output: &str) -> Option<HostReady> {
    let mut fingerprint = None;
    let mut port = None;
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("Fingerprint:") {
            fingerprint = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("Listening:") {
            port = value.trim().parse::<SocketAddr>().ok().map(|a| a.port());
        }
    }
    Some(HostReady {
        fingerprint: fingerprint.filter(|f| !f.is_empty())?,
        port: port?,
    })
}

/// Arguments of `dazzdesk host`: only `client` may connect, the windows of
/// `app` move onto the virtual display and are the only ones shared, and
/// the host's own displays stay on so the machine remains usable.
fn host_args(client: &str, app: &str) -> Vec<String> {
    [
        "host",
        "--allow",
        client,
        "--filter",
        app,
        "--only",
        app,
        "--keep-displays",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// `address:port`, bracketing IPv6 addresses.
fn socket_address(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// `remote-agent dazzdesk-host`, on the host: starts `dazzdesk host` in the
/// desktop session, prints a [`HostReady`] line, and stops the Host when
/// stdin closes (the client is done or the connection dropped).
pub fn serve_host(client: &str, filter: &str) -> Result<()> {
    if !cfg!(windows) {
        bail!("`--remote-window` supports Windows hosts only for now");
    }
    let exe = tokio::runtime::Handle::current().block_on(ensure_dazzdesk())?;
    // Info-level logs say what happened to the virtual displays and
    // windows; only warnings and errors are relayed.
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let host = run_desktop::launch(
        "window",
        &exe,
        &host_args(client, filter),
        &[("RUST_LOG", &log_level)],
        &std::env::temp_dir(),
        false,
    )?;
    let _keep_log = KeepLog(&host);

    let started = Instant::now();
    let ready = loop {
        let (out, err) = host.output();
        if let Some(ready) = parse_host_output(&out) {
            break ready;
        }
        if !host.is_running() {
            // WSAEADDRINUSE: usually a dazzdesk host started by hand.
            let hint = if err.contains("os error 10048") {
                "Port 47100 is in use; is a remote window from another run still open?\n"
            } else {
                ""
            };
            bail!("The remote window could not start.\n{hint}{out}{err}");
        }
        if started.elapsed() > HOST_START_TIMEOUT {
            bail!("The remote window did not start in time.\n{out}{err}");
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &ready)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    drop(stdout);

    let closed = Arc::new(AtomicBool::new(false));
    std::thread::spawn({
        let closed = closed.clone();
        move || {
            let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
            closed.store(true, Ordering::SeqCst);
        }
    });
    // The Host logs problems (e.g. no virtual display) to stderr; relay
    // them, since its window otherwise never shows up without a word.
    let mut relayed = host.output().1.len();
    while !closed.load(Ordering::SeqCst) {
        let (out, err) = host.output();
        if let Some(new) = err.get(relayed..)
            && let Some(end) = new.rfind('\n')
        {
            relay_lines(&new[..end]);
            relayed += end + 1;
        }
        if !host.is_running() {
            bail!("The remote window stopped.\n{out}{err}");
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}

/// Where the last Host's log is kept after it stops.
fn host_log_path() -> std::path::PathBuf {
    std::env::temp_dir().join("fastforge-remote-window.log")
}

/// Copies the Host's log to [`host_log_path`] when dropped, since its
/// launch directory is removed with it.
struct KeepLog<'a>(&'a run_desktop::DesktopApp);

impl Drop for KeepLog<'_> {
    fn drop(&mut self) {
        let (out, err) = self.0.output();
        let _ = std::fs::write(host_log_path(), format!("{out}{err}"));
    }
}

/// Whether a Host log line is a warning or an error.
fn is_problem(line: &str) -> bool {
    line.contains(" WARN") || line.contains("ERROR")
}

/// Prefix of the Host's problems on the agent's stderr; the client shows
/// only these lines and errors.
const RELAY_PREFIX: &str = "remote window: ";

/// A Host log line without colors and the logging target:
/// `WARN some::module: message` becomes `message`.
fn clean_log_line(line: &str) -> String {
    let mut plain = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip a CSI sequence: ESC [ … final byte.
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            plain.push(c);
        }
    }
    let trimmed = plain.trim();
    let rest = ["ERROR", "WARN"]
        .iter()
        .find_map(|level| trimmed.strip_prefix(level))
        .map(str::trim_start);
    match rest.and_then(|rest| rest.split_once(": ")) {
        Some((target, message)) if !target.contains(' ') => message.to_string(),
        _ => trimmed.to_string(),
    }
}

/// Writes the warnings and errors in `text` to stderr with CRLF endings,
/// which also read correctly on the client's terminal while it is in raw
/// mode.
fn relay_lines(text: &str) {
    let mut stderr = std::io::stderr().lock();
    for line in text.lines().filter(|line| is_problem(line)) {
        let _ = write!(stderr, "{RELAY_PREFIX}{}\r\n", clean_log_line(line));
    }
    let _ = stderr.flush();
}

/// The fingerprint `dazzdesk identity --json` prints.
fn local_fingerprint(dazzdesk: &Path) -> Result<String> {
    #[derive(Deserialize)]
    struct Identity {
        fingerprint: String,
    }
    let output = Command::new(dazzdesk)
        .args(["identity", "--json"])
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .context("Could not read this machine's remote window identity")?;
    if !output.status.success() {
        bail!(
            "Could not read this machine's remote window identity ({})",
            output.status
        );
    }
    let identity: Identity = serde_json::from_slice(&output.stdout)
        .context("Could not read this machine's remote window identity")?;
    Ok(identity.fingerprint)
}

/// Keeps a child out of this terminal's process group, so Ctrl-C reaches
/// only the run; the child is stopped when [`LocalDisplay`] is dropped.
fn detach(command: &mut Command) -> &mut Command {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    command
}

/// Whether a line of the run's output says the app is running: the
/// Windows launch in the desktop session (any build mode), or a Dart VM
/// service.
pub fn app_started(line: &str) -> bool {
    line.contains("in the desktop session (pid") || line.contains("Dart VM Service")
}

/// Prints a status line from a background thread while the run owns the
/// terminal, which may be in raw mode.
fn say(raw: bool, message: &str) {
    if raw {
        notice(true, &message.replace('\n', "\r\n"));
    } else {
        notice(false, message);
    }
}

/// `--remote-window`: checked before the run, and started in the background
/// once the run's output shows the app running.
pub struct RemoteWindow {
    host: HostConfig,
    app: String,
    dazzdesk: PathBuf,
    fingerprint: String,
    state: Arc<Mutex<WindowState>>,
}

#[derive(Default)]
struct WindowState {
    requested: bool,
    stopped: bool,
    display: Option<LocalDisplay>,
}

impl RemoteWindow {
    /// Checks that the window of the app run from the current directory for
    /// `platform` on `remote` can be shown here, and sets up both machines.
    pub async fn prepare(remote: &RemoteClient, platform: Option<&str>) -> Result<Self> {
        if !cfg!(any(target_os = "macos", windows)) {
            bail!("`--remote-window` needs macOS or Windows on this machine");
        }
        if remote.host().os() != RemoteOs::Windows {
            bail!(
                "`--remote-window` supports Windows hosts only for now; `{}` isn't one",
                remote.host().name
            );
        }
        if platform != Some("windows") {
            bail!("`--remote-window` shows Windows desktop apps only for now; pass `-p windows`");
        }
        let cmake = std::fs::read_to_string("windows/CMakeLists.txt").context(
            "windows/CMakeLists.txt not found; is this a Flutter project with Windows support?",
        )?;
        let app = run_desktop::binary_name(&cmake)
            .ok_or_else(|| anyhow!("BINARY_NAME not set in windows/CMakeLists.txt"))?;
        let dazzdesk = ensure_dazzdesk().await?;
        let fingerprint = local_fingerprint(&dazzdesk)?;
        let status = remote
            .agent_command(&["remote-window-setup"])
            .stdin(Stdio::null())
            .status()
            .context("Failed to start the transport")?;
        if !status.success() {
            bail!(
                "Could not set up remote windows on `{}` ({status}); is its fastforge recent enough?",
                remote.host().name
            );
        }
        Ok(Self {
            host: remote.host().clone(),
            app,
            dazzdesk,
            fingerprint,
            state: Arc::default(),
        })
    }

    /// Takes each line of the run's output; the first that shows the app
    /// running starts dazzdesk in the background.
    pub fn observe(&self, line: &str, raw: bool) {
        if !app_started(line) {
            return;
        }
        {
            let mut state = self.state.lock().unwrap();
            if state.requested || state.stopped {
                return;
            }
            state.requested = true;
        }
        let (host, app, dazzdesk, fingerprint) = (
            self.host.clone(),
            self.app.clone(),
            self.dazzdesk.clone(),
            self.fingerprint.clone(),
        );
        let state = self.state.clone();
        std::thread::spawn(move || {
            let remote = RemoteClient::new(host);
            match LocalDisplay::start(&remote, &app, &dazzdesk, &fingerprint, raw) {
                Ok(display) => {
                    let mut state = state.lock().unwrap();
                    // The run may have ended while dazzdesk started.
                    if !state.stopped {
                        state.display = Some(display);
                    }
                }
                Err(error) => say(raw, &format!("remote window: {error:#}")),
            }
        });
    }

    /// Stops dazzdesk on both machines, or keeps it from starting.
    pub fn stop(&self) {
        let display = {
            let mut state = self.state.lock().unwrap();
            state.stopped = true;
            state.display.take()
        };
        drop(display);
    }
}

/// The app's window shown here: `dazzdesk host` running on the host and
/// `dazzdesk client` here. Both stop when this is dropped.
struct LocalDisplay {
    agent: Child,
    agent_stdin: Option<ChildStdin>,
    client: Child,
}

impl LocalDisplay {
    /// Starts the Host on `remote` sharing the windows of `app` with the
    /// Client `fingerprint`, then the Client here. Status goes through
    /// [`say`].
    fn start(
        remote: &RemoteClient,
        app: &str,
        dazzdesk: &Path,
        fingerprint: &str,
        raw: bool,
    ) -> Result<Self> {
        let host_name = &remote.host().name;
        say(raw, &format!("opening the window from {host_name} here"));
        let mut agent = detach(&mut remote.agent_command(&[
            "remote-window",
            "--client",
            fingerprint,
            "--filter",
            app,
        ]))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start the transport")?;
        // The Host's problems and the agent's errors, as status lines.
        let stderr = agent.stderr.take().expect("piped stderr");
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let line = line.trim_end();
                if line.starts_with(RELAY_PREFIX) || line.starts_with("Error:") {
                    say(raw, line);
                }
            }
        });
        let agent_stdin = agent.stdin.take();
        let mut line = String::new();
        BufReader::new(agent.stdout.take().expect("piped stdout")).read_line(&mut line)?;
        let ready: HostReady = match serde_json::from_str(&line) {
            Ok(ready) => ready,
            Err(_) => {
                drop(agent_stdin);
                let status = agent.wait()?;
                bail!("Could not open the window from `{host_name}` ({status})");
            }
        };

        let address = socket_address(&remote.network_host()?, ready.port);
        let log_path = std::env::temp_dir().join(format!(
            "fastforge-remote-window-{}.log",
            std::process::id()
        ));
        let log = File::create(&log_path)
            .with_context(|| format!("Failed to create {}", log_path.display()))?;
        let client = detach(Command::new(dazzdesk).args([
            "client",
            &address,
            "--host-fingerprint",
            &ready.fingerprint,
        ]))
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()
        .context("Could not show the remote window here")?;
        let mut display = Self {
            agent,
            agent_stdin,
            client,
        };
        if display.wait_for_client(&address, &log_path)? {
            say(
                raw,
                &format!("showing {app}'s window here (log: {})", log_path.display()),
            );
        } else {
            say(
                raw,
                &format!(
                    "the window from {address} hasn't connected yet (see {})",
                    log_path.display()
                ),
            );
        }
        Ok(display)
    }

    /// Waits until the Client says it connected (`true`) or the timeout
    /// passes (`false`), failing if it exits.
    fn wait_for_client(&mut self, address: &str, log: &Path) -> Result<bool> {
        let started = Instant::now();
        loop {
            let mut output = String::new();
            if let Ok(mut file) = File::open(log) {
                let _ = file.read_to_string(&mut output);
            }
            if output.contains("Connected:") {
                return Ok(true);
            }
            if let Some(status) = self.client.try_wait()? {
                bail!(
                    "The window from {address} could not connect ({status}). This machine must reach \
                     UDP port {} on the host directly (it isn't tunnelled through SSH), and Windows \
                     Firewall there must allow it.\n{output}",
                    address.rsplit(':').next().unwrap_or_default()
                );
            }
            if started.elapsed() > CLIENT_CONNECT_TIMEOUT {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

impl Drop for LocalDisplay {
    fn drop(&mut self) {
        let _ = self.client.kill();
        let _ = self.client.wait();
        // Closing stdin tells the agent to stop the Host.
        drop(self.agent_stdin.take());
        for _ in 0..30 {
            if !matches!(self.agent.try_wait(), Ok(None)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.agent.kill();
        let _ = self.agent.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_the_host_listens_on() {
        let output = "Fingerprint: ab12cd\nListening:   0.0.0.0:47100\n";
        assert_eq!(
            parse_host_output(output),
            Some(HostReady {
                fingerprint: "ab12cd".into(),
                port: 47100
            })
        );
        assert_eq!(parse_host_output("Fingerprint: ab12cd\n"), None);
        assert_eq!(parse_host_output("Listening:   [::]:47200\n"), None);
        assert_eq!(
            parse_host_output("Fingerprint: x\nListening:   [::]:47200\n").map(|r| r.port),
            Some(47200)
        );
    }

    #[test]
    fn names_the_pinned_release_archives() {
        let (name, url) = dazzdesk_archive("x86_64-pc-windows-msvc");
        assert_eq!(
            name,
            format!("dazzdesk-{DAZZDESK_VERSION}-x86_64-pc-windows-msvc.zip")
        );
        assert_eq!(
            url,
            format!(
                "https://github.com/dazzlabs/dazzdesk/releases/download/v{DAZZDESK_VERSION}/{name}"
            )
        );
        assert!(
            dazzdesk_archive("aarch64-apple-darwin")
                .0
                .ends_with("-aarch64-apple-darwin.tar.gz")
        );
        let path = managed_dazzdesk().unwrap();
        assert!(
            path.ends_with(
                std::path::Path::new(".fastforge/tools/dazzdesk")
                    .join(DAZZDESK_VERSION)
                    .join(dazzdesk_file_name())
            )
        );
    }

    #[test]
    fn notices_when_the_app_runs() {
        assert!(app_started(
            r"Started C:\w\build\windows\x64\runner\Debug\winapp.exe in the desktop session (pid 4242)."
        ));
        assert!(app_started(
            "A Dart VM Service on Windows is available at: http://127.0.0.1:62311/abc=/"
        ));
        assert!(!app_started("Building Windows application..."));
        assert!(!app_started(
            "This session has no desktop; the app will open on the logged-in user's desktop."
        ));
    }

    #[test]
    fn cleans_host_log_lines() {
        assert_eq!(
            clean_log_line(
                "\u{1b}[33m WARN\u{1b}[0m \u{1b}[2mdazzdesk_session::host\u{1b}[0m\u{1b}[2m:\u{1b}[0m configuring virtual displays failed"
            ),
            "configuring virtual displays failed"
        );
        assert_eq!(clean_log_line("ERROR x: a: b"), "a: b");
        assert_eq!(clean_log_line("plain text"), "plain text");
    }

    #[test]
    fn relays_only_warnings_and_errors() {
        assert!(is_problem(
            "\u{1b}[33m WARN\u{1b}[0m dazzdesk_session::host: configuring virtual displays failed"
        ));
        assert!(is_problem("ERROR dazzdesk: boom"));
        assert!(!is_problem(
            "\u{1b}[32m INFO\u{1b}[0m dazzdesk_session::host: client connected"
        ));
    }

    #[test]
    fn builds_host_arguments_and_addresses() {
        assert_eq!(
            host_args("fp", "winapp"),
            [
                "host",
                "--allow",
                "fp",
                "--filter",
                "winapp",
                "--only",
                "winapp",
                "--keep-displays"
            ]
        );
        assert_eq!(socket_address("192.168.1.20", 47100), "192.168.1.20:47100");
        assert_eq!(socket_address("win.local", 1), "win.local:1");
        assert_eq!(socket_address("fe80::1", 47100), "[fe80::1]:47100");
    }
}
