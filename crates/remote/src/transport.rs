//! Turns a remote shell command into a local process.

use crate::hosts::{HostConfig, TransportKind};
use crate::shell::quote;
use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub trait Transport: Send + Sync {
    /// A command that runs the POSIX shell command `script` on the host.
    /// `tty` requests a terminal (interactive commands).
    fn command(&self, script: &str, tty: bool) -> Command;

    /// Wraps `script` so it runs in the user's login shell, which is where
    /// `PATH` additions for Flutter, Homebrew etc. usually live.
    fn login_script(&self, script: &str) -> String {
        format!("exec \"${{SHELL:-/bin/sh}}\" -lc {}", quote(script))
    }

    /// A long-running command forwarding local `port` to the same port on the
    /// host's loopback interface; `None` when the host is this machine.
    fn forward_port(&self, _port: u16) -> Option<Command> {
        None
    }

    /// The name or address the host is reached at over the network.
    fn network_host(&self) -> Result<String>;
}

pub fn for_host(host: &HostConfig) -> Box<dyn Transport> {
    match host.transport {
        TransportKind::Ssh => Box::new(SshTransport::new(host)),
        TransportKind::Local => Box::new(LocalTransport),
    }
}

/// The system OpenSSH client.
pub struct SshTransport {
    args: Vec<String>,
    destination: String,
}

impl SshTransport {
    pub fn new(host: &HostConfig) -> Self {
        let mut args = vec!["-o".to_string(), "ServerAliveInterval=30".to_string()];
        if let Some(port) = host.port {
            args.push("-p".into());
            args.push(port.to_string());
        }
        if let Some(user) = &host.user {
            args.push("-l".into());
            args.push(user.clone());
        }
        if let Some(identity) = &host.identity_file {
            args.push("-i".into());
            args.push(expand_local_home(identity).to_string_lossy().into_owned());
        }
        if let Some(dir) = control_dir() {
            // Reuse one connection for the several ssh calls a command makes.
            args.extend([
                "-o".to_string(),
                "ControlMaster=auto".to_string(),
                "-o".to_string(),
                format!("ControlPath={}/%C", dir.display()),
                "-o".to_string(),
                "ControlPersist=60".to_string(),
            ]);
        }
        Self {
            args,
            destination: host.host.clone().unwrap_or_default(),
        }
    }

    pub fn args(&self, script: &str, tty: bool) -> Vec<String> {
        let mut args = self.args.clone();
        if tty {
            // Hide "Shared connection to … closed." on interactive sessions.
            args.extend(["-o".to_string(), "LogLevel=ERROR".to_string()]);
        }
        args.push(if tty { "-tt" } else { "-T" }.to_string());
        args.push(self.destination.clone());
        args.push(script.to_string());
        args
    }
}

impl Transport for SshTransport {
    fn command(&self, script: &str, tty: bool) -> Command {
        let mut command = Command::new("ssh");
        command.args(self.args(script, tty));
        command
    }

    fn forward_port(&self, port: u16) -> Option<Command> {
        let mut command = Command::new("ssh");
        // ssh keeps the first value of an option, so these win over the
        // shared-connection settings: a forward owned by a master connection
        // would outlive the run and block the port next time.
        command
            .args([
                "-o",
                "ControlPath=none",
                "-o",
                "ExitOnForwardFailure=yes",
                "-N",
            ])
            .arg("-L")
            .arg(format!("{port}:127.0.0.1:{port}"))
            .args(&self.args)
            .arg(&self.destination);
        Some(command)
    }

    fn network_host(&self) -> Result<String> {
        let output = Command::new("ssh")
            .arg("-G")
            .args(&self.args)
            .arg(&self.destination)
            .stdin(Stdio::null())
            .output()
            .context("Failed to run `ssh -G`")?;
        ssh_config_hostname(&String::from_utf8_lossy(&output.stdout))
            .ok_or_else(|| anyhow!("`ssh -G {}` printed no hostname", self.destination))
    }
}

/// The `hostname` line of `ssh -G` output.
pub fn ssh_config_hostname(config: &str) -> Option<String> {
    config.lines().find_map(|line| {
        let value = line.strip_prefix("hostname ")?.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// `~/.fastforge/ssh`, created with private permissions. Connection sharing
/// is not available with the Windows OpenSSH client.
fn control_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        return None;
    }
    let dir = dirs::home_dir()?.join(".fastforge").join("ssh");
    std::fs::create_dir_all(&dir).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).ok()?;
    }
    Some(dir)
}

fn expand_local_home(path: &str) -> PathBuf {
    crate::shell::expand_home(path, dirs::home_dir().as_deref())
}

/// Runs the "remote" side on this machine with `sh`, for tests.
pub struct LocalTransport;

impl Transport for LocalTransport {
    fn command(&self, script: &str, _tty: bool) -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(script);
        command
    }

    fn login_script(&self, script: &str) -> String {
        script.to_string()
    }

    fn network_host(&self) -> Result<String> {
        Ok("127.0.0.1".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_ssh_arguments() {
        let mut host = HostConfig::new("mac");
        host.set_destination("builder@mac.local:2222").unwrap();
        let transport = SshTransport::new(&host);
        let args = transport.args("echo hi", false);
        let joined = args.join(" ");
        assert!(joined.contains("-p 2222"));
        assert!(joined.contains("-l builder"));
        assert!(joined.ends_with("-T mac.local echo hi"));
        assert!(transport.args("x", true).contains(&"-tt".to_string()));

        let forward = transport.forward_port(9100).unwrap();
        let args: Vec<String> = forward
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[..2], ["-o", "ControlPath=none"]);
        assert!(args.join(" ").contains("-L 9100:127.0.0.1:9100"));
        assert_eq!(args.last().unwrap(), "mac.local");
        assert!(LocalTransport.forward_port(1).is_none());
    }

    #[test]
    fn reads_the_hostname_from_ssh_config() {
        let config = "user builder\nhostname 192.168.1.20\nport 22\n";
        assert_eq!(ssh_config_hostname(config).as_deref(), Some("192.168.1.20"));
        assert_eq!(ssh_config_hostname("user x\n"), None);
        assert_eq!(
            LocalTransport.network_host().unwrap(),
            "127.0.0.1".to_string()
        );
    }

    #[test]
    fn login_script_quotes_the_command() {
        let transport = LocalTransport;
        assert_eq!(transport.login_script("a 'b'"), "a 'b'");
        let host = {
            let mut h = HostConfig::new("h");
            h.host = Some("x".into());
            h
        };
        let ssh = SshTransport::new(&host);
        assert_eq!(
            ssh.login_script("cd /w && ls"),
            "exec \"${SHELL:-/bin/sh}\" -lc 'cd /w && ls'"
        );
    }
}
