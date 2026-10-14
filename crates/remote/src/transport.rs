//! Turns a remote shell command into a local process.

use crate::hosts::{HostConfig, TransportKind};
use crate::shell::quote;
use anyhow::{Context, Result, anyhow, bail};
use std::collections::BTreeSet;
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;

/// A local loopback TCP port forwarded to a port on the host's loopback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortForward {
    local_port: NonZeroU16,
    remote_port: NonZeroU16,
}

impl PortForward {
    pub fn local_port(self) -> u16 {
        self.local_port.get()
    }

    pub fn remote_port(self) -> u16 {
        self.remote_port.get()
    }

    fn ssh_spec(self) -> String {
        format!(
            "127.0.0.1:{}:127.0.0.1:{}",
            self.local_port, self.remote_port
        )
    }
}

impl FromStr for PortForward {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let (local, remote) = value.split_once(':').unwrap_or((value, value));
        let parse = |port: &str| {
            if !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) {
                port.parse::<NonZeroU16>().ok()
            } else {
                None
            }
        };
        match (parse(local), parse(remote)) {
            (Some(local_port), Some(remote_port)) => Ok(Self {
                local_port,
                remote_port,
            }),
            _ => bail!(
                "Invalid port forward `{value}`; expected PORT or LOCAL:REMOTE with ports in 1..=65535"
            ),
        }
    }
}

pub trait Transport: Send + Sync {
    /// A command that runs the POSIX shell command `script` on the host.
    /// `tty` requests a terminal (interactive commands).
    fn command(&self, script: &str, tty: bool) -> Command;

    /// Runs a command with TCP forwards owned by that connection, so they
    /// close when the command ends. Only SSH supports port forwarding.
    fn command_with_forwards(
        &self,
        script: &str,
        tty: bool,
        forwards: &[PortForward],
    ) -> Result<Command> {
        if !forwards.is_empty() {
            bail!("Port forwarding requires an SSH host");
        }
        Ok(self.command(script, tty))
    }

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

    fn command_with_forwards(
        &self,
        script: &str,
        tty: bool,
        forwards: &[PortForward],
    ) -> Result<Command> {
        if forwards.is_empty() {
            return Ok(self.command(script, tty));
        }
        let mut local_ports = BTreeSet::new();
        let mut command = Command::new("ssh");
        // These must precede the shared connection options: forwards on a
        // persistent master would survive the command and keep ports bound.
        command.args(["-o", "ControlPath=none", "-o", "ExitOnForwardFailure=yes"]);
        for forward in forwards {
            if !local_ports.insert(forward.local_port) {
                bail!(
                    "Local port {} is forwarded more than once",
                    forward.local_port
                );
            }
            command.arg("-L").arg(forward.ssh_spec());
        }
        command.args(self.args(script, tty));
        Ok(command)
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
    fn parses_port_forwards_and_rejects_invalid_ports() {
        let same: PortForward = "8080".parse().unwrap();
        assert_eq!((same.local_port(), same.remote_port()), (8080, 8080));
        let mapped: PortForward = "18080:8080".parse().unwrap();
        assert_eq!((mapped.local_port(), mapped.remote_port()), (18080, 8080));
        assert!("1:65535".parse::<PortForward>().is_ok());
        for value in [
            "",
            "0",
            "65536",
            "0:80",
            "80:0",
            "80:65536",
            ":80",
            "80:",
            "1:2:3",
            "abc",
            "+80",
            " 80",
            "80:localhost",
        ] {
            assert!(value.parse::<PortForward>().is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn shell_forwards_belong_to_the_command_connection() {
        let mut host = HostConfig::new("mac");
        host.set_destination("builder@mac.local:2222").unwrap();
        host.identity_file = Some("/tmp/key".into());
        let transport = SshTransport::new(&host);
        let forwards = ["8080".parse().unwrap(), "19000:9000".parse().unwrap()];
        for tty in [false, true] {
            let command = transport
                .command_with_forwards("serve", tty, &forwards)
                .unwrap();
            let args: Vec<_> = command
                .get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect();
            assert_eq!(
                &args[..8],
                &[
                    "-o",
                    "ControlPath=none",
                    "-o",
                    "ExitOnForwardFailure=yes",
                    "-L",
                    "127.0.0.1:8080:127.0.0.1:8080",
                    "-L",
                    "127.0.0.1:19000:127.0.0.1:9000",
                ]
            );
            assert!(args.windows(2).any(|args| args == ["-p", "2222"]));
            assert!(args.windows(2).any(|args| args == ["-l", "builder"]));
            assert!(args.windows(2).any(|args| args == ["-i", "/tmp/key"]));
            assert_eq!(
                &args[args.len() - 3..],
                &[if tty { "-tt" } else { "-T" }, "mac.local", "serve"]
            );
        }
        let command = transport
            .command_with_forwards("serve", false, &[])
            .unwrap();
        let args: Vec<_> = command.get_args().collect();
        let ordinary = transport.command("serve", false);
        assert_eq!(args, ordinary.get_args().collect::<Vec<_>>());
    }

    #[test]
    fn rejects_duplicate_local_ports_and_non_ssh_forwards() {
        let mut host = HostConfig::new("mac");
        host.host = Some("mac.local".into());
        let transport = SshTransport::new(&host);
        for specs in [["8080", "8080"], ["8080:80", "8080:90"]] {
            let forwards: Vec<PortForward> = specs.iter().map(|s| s.parse().unwrap()).collect();
            let error = transport
                .command_with_forwards("serve", false, &forwards)
                .unwrap_err();
            assert!(error.to_string().contains("Local port 8080"));
        }
        assert!(
            LocalTransport
                .command_with_forwards("serve", false, &["8080".parse().unwrap()])
                .is_err()
        );
        assert!(
            LocalTransport
                .command_with_forwards("true", false, &[])
                .is_ok()
        );
    }

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
