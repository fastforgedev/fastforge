//! Interactive remote runs (`fastforge run --host`): the client relays the
//! local terminal to `remote-agent attach`, syncs the project before each hot
//! reload/restart key, and forwards the loopback ports the run prints (VM
//! service, DevTools, web server) so the URLs work locally.

use crate::client::{LocalProject, RemoteClient};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::io::{IsTerminal, Read, Write};
use std::process::{Child, Command, Stdio};

/// Keys that make `flutter run` re-read the sources.
const RELOAD_KEYS: &[u8] = b"rR";

/// Loopback URL prefixes whose ports are forwarded.
const LOOPBACK_PREFIXES: &[&str] = &[
    "http://127.0.0.1:",
    "http://localhost:",
    "ws://127.0.0.1:",
    "ws://localhost:",
    "http://[::1]:",
];

/// Where a project is synced, for re-syncing on reload.
pub struct ResyncTarget {
    pub project: LocalProject,
    pub workspace: String,
    pub excludes: Vec<String>,
}

/// Ports of loopback URLs in `line`, in order of appearance.
pub fn loopback_ports(line: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for prefix in LOOPBACK_PREFIXES {
        let mut rest = line;
        while let Some(index) = rest.find(prefix) {
            rest = &rest[index + prefix.len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(port) = digits.parse::<u16>()
                && port > 0
                && !ports.contains(&port)
            {
                ports.push(port);
            }
        }
    }
    ports
}

/// Puts the local terminal in raw mode until dropped, so keys reach the
/// remote terminal unprocessed (including Ctrl-C).
struct RawTerminal {
    saved: String,
}

impl RawTerminal {
    fn enable() -> Option<Self> {
        let output = Command::new("stty")
            .arg("-g")
            .stdin(Stdio::inherit())
            .output()
            .ok()?;
        let saved = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !output.status.success() || saved.is_empty() {
            return None;
        }
        let ok = Command::new("stty")
            .args(["raw", "-echo"])
            .stdin(Stdio::inherit())
            .status()
            .is_ok_and(|s| s.success());
        ok.then_some(Self { saved })
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        let _ = Command::new("stty")
            .arg(&self.saved)
            .stdin(Stdio::inherit())
            .status();
    }
}

/// `rows cols` of the local terminal.
fn terminal_size() -> Option<(u16, u16)> {
    let output = Command::new("stty")
        .arg("size")
        .stdin(Stdio::inherit())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace().map(|p| p.parse::<u16>().ok());
    Some((parts.next()??, parts.next()??))
}

/// Shell commands that prepare the remote terminal. Its settings come from
/// our stdin, which is a pipe: ssh then sends no terminal modes, and over a
/// shared (ControlMaster) connection the terminal even starts with all modes
/// off, so output lacks carriage returns and Ctrl-C raises no signal.
/// `stty sane` restores the defaults; the size is copied from our terminal.
fn remote_tty_setup(size: Option<(u16, u16)>) -> String {
    match size {
        Some((rows, cols)) => format!("stty sane rows {rows} cols {cols} 2>/dev/null; "),
        None => "stty sane 2>/dev/null; ".to_string(),
    }
}

/// Prints a fastforge status line; raw mode needs explicit carriage returns.
fn notice(raw: bool, message: &str) {
    let mut stderr = std::io::stderr();
    if raw {
        let _ = write!(stderr, "\r\n\x1b[90m[fastforge] {message}\x1b[0m\r\n");
    } else {
        let _ = writeln!(stderr, "[fastforge] {message}");
    }
    let _ = stderr.flush();
}

impl RemoteClient {
    /// Attaches the local terminal to the interactive run `run_id` stored by
    /// a previous sync, and returns its exit code.
    pub fn attach_interactive(
        &self,
        workspace: &str,
        run_id: &str,
        resync: Option<ResyncTarget>,
    ) -> Result<i32> {
        let terminal =
            cfg!(unix) && std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
        let mut script = self.agent_script(&["attach", "--workspace", workspace, "--run", run_id]);
        if terminal && self.host.os() == crate::hosts::RemoteOs::Unix {
            script = format!("{}{script}", remote_tty_setup(terminal_size()));
        }
        let mut child = self
            .transport
            .command(&script, terminal)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to start the transport")?;
        let raw = if terminal {
            RawTerminal::enable()
        } else {
            None
        };
        let is_raw = raw.is_some();

        // Input: local keys → remote, syncing first on reload keys. The thread
        // is left blocked on stdin when the run ends; the process exits soon after.
        let mut remote_stdin = child.stdin.take().expect("piped stdin");
        let host = self.host.clone();
        std::thread::spawn(move || {
            let syncer = resync.map(|target| (RemoteClient::new(host), target));
            let mut stdin = std::io::stdin();
            let mut buf = [0u8; 1024];
            loop {
                let n = match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                if let Some((client, target)) = &syncer
                    && buf[..n].iter().any(|b| RELOAD_KEYS.contains(b))
                {
                    match client.resync(&target.project, &target.workspace, &target.excludes) {
                        Ok(stats) if stats.changed + stats.deleted > 0 => notice(
                            is_raw,
                            &format!(
                                "synced {} changed, {} deleted",
                                stats.changed, stats.deleted
                            ),
                        ),
                        Ok(_) => {}
                        Err(error) => notice(is_raw, &format!("sync failed: {error:#}")),
                    }
                }
                if remote_stdin.write_all(&buf[..n]).is_err() || remote_stdin.flush().is_err() {
                    break;
                }
            }
        });

        // Output: remote → local, forwarding the loopback ports it mentions.
        let mut remote_stdout = child.stdout.take().expect("piped stdout");
        let mut stdout = std::io::stdout();
        let mut forwarders: Vec<Child> = Vec::new();
        let mut forwarded = BTreeSet::new();
        let mut line = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = match remote_stdout.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let _ = stdout.write_all(&buf[..n]);
            let _ = stdout.flush();
            for &byte in &buf[..n] {
                if byte != b'\n' {
                    if line.len() < 16 * 1024 {
                        line.push(byte);
                    }
                    continue;
                }
                for port in loopback_ports(&String::from_utf8_lossy(&line)) {
                    if !forwarded.insert(port) {
                        continue;
                    }
                    let Some(mut command) = self.transport.forward_port(port) else {
                        continue;
                    };
                    match command
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        Ok(forwarder) => {
                            forwarders.push(forwarder);
                            notice(
                                is_raw,
                                &format!("forwarding localhost:{port} to {}", self.host.name),
                            );
                        }
                        Err(error) => {
                            notice(is_raw, &format!("could not forward port {port}: {error}"))
                        }
                    }
                }
                line.clear();
            }
        }

        let status = child.wait()?;
        for mut forwarder in forwarders {
            let _ = forwarder.kill();
            let _ = forwarder.wait();
        }
        drop(raw);
        Ok(status.code().unwrap_or(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resets_the_remote_terminal() {
        assert_eq!(
            remote_tty_setup(Some((40, 120))),
            "stty sane rows 40 cols 120 2>/dev/null; "
        );
        assert_eq!(remote_tty_setup(None), "stty sane 2>/dev/null; ");
    }

    #[test]
    fn finds_loopback_ports() {
        assert_eq!(
            loopback_ports(
                "A Dart VM Service on macOS is available at: http://127.0.0.1:54321/abc=/"
            ),
            vec![54321]
        );
        assert_eq!(
            loopback_ports("DevTools at: http://127.0.0.1:9100?uri=http://127.0.0.1:54321/abc=/"),
            vec![9100, 54321]
        );
        assert_eq!(
            loopback_ports("lib/main.dart is being served at http://localhost:8080"),
            vec![8080]
        );
        assert!(loopback_ports("http://example.com:80 http://127.0.0.1:x").is_empty());
        assert!(loopback_ports("http://127.0.0.1:99999").is_empty());
    }
}
