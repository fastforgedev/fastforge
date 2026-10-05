//! Remote host definitions, stored per machine in `~/.fastforge/hosts.yaml`.
//!
//! Host addresses, user names and key paths are machine specific, so they are
//! never part of a project: `distribute_options.yaml` only refers to hosts by
//! name.

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Environment variable overriding the location of the hosts file.
pub const HOSTS_FILE_ENV: &str = "FASTFORGE_HOSTS_FILE";

/// Default remote directory the project workspaces are synced into.
pub const DEFAULT_WORKDIR: &str = "~/.fastforge/remote/workspaces";

/// How fastforge reaches a host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportKind {
    /// The system `ssh` client (honours `~/.ssh/config`, ssh-agent, ProxyJump).
    #[default]
    Ssh,
    /// Runs the "remote" side on this machine through `sh`. Meant for testing
    /// the remote pipeline without an SSH server.
    Local,
}

/// The host's operating system family, which decides how remote commands
/// are written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteOs {
    /// macOS or Linux: commands run in the user's POSIX login shell.
    #[default]
    Unix,
    /// Windows with OpenSSH's default shell, `cmd.exe`.
    Windows,
}

impl RemoteOs {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unix => "unix",
            Self::Windows => "windows",
        }
    }
}

impl TransportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Local => "local",
        }
    }
}

/// One remote host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostConfig {
    pub name: String,
    #[serde(default)]
    pub transport: TransportKind,
    /// `unix` or `windows`; detected by `fastforge host doctor` when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<RemoteOs>,
    /// Host name, address or `~/.ssh/config` alias. Unused by the `local`
    /// transport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<String>,
    /// Remote directory the workspaces are synced into
    /// (default `~/.fastforge/remote/workspaces`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    /// Path of the remote `fastforge` binary (default: found on the remote
    /// login shell's `PATH`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fastforge: Option<String>,
    /// Platforms this host builds for. Filled in by `fastforge host doctor`
    /// when left empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
    /// Local environment variables forwarded to remote runs. Entries may end
    /// with `*` to match a prefix (e.g. `APPLE_*`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forward_env: Vec<String>,
    /// Environment variables set for remote runs. Values may reference remote
    /// variables (`$HOME`, `${PATH}`) and start with `~/`; they are expanded
    /// on the remote host.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

impl HostConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: TransportKind::Ssh,
            os: None,
            host: None,
            user: None,
            port: None,
            identity_file: None,
            workdir: None,
            fastforge: None,
            platforms: Vec::new(),
            forward_env: Vec::new(),
            env: BTreeMap::new(),
        }
    }

    /// Applies an SSH destination of the form `[user@]host[:port]`.
    pub fn set_destination(&mut self, destination: &str) -> Result<()> {
        let (user, rest) = match destination.rsplit_once('@') {
            Some((user, rest)) => (Some(user), rest),
            None => (None, destination),
        };
        // Bracketed IPv6 addresses (`[::1]:22`) keep their colons.
        let (host, port) = if let Some(stripped) = rest.strip_prefix('[') {
            let (host, tail) = stripped
                .split_once(']')
                .ok_or_else(|| anyhow!("Invalid destination `{destination}`"))?;
            (host, tail.strip_prefix(':'))
        } else {
            match rest.split_once(':') {
                Some((host, port)) if !port.contains(':') => (host, Some(port)),
                _ => (rest, None),
            }
        };
        if host.is_empty() || user.is_some_and(str::is_empty) {
            bail!("Invalid destination `{destination}`, expected [user@]host[:port]");
        }
        self.host = Some(host.to_string());
        if let Some(user) = user {
            self.user = Some(user.to_string());
        }
        if let Some(port) = port {
            self.port = Some(
                port.parse()
                    .with_context(|| format!("Invalid port in destination `{destination}`"))?,
            );
        }
        Ok(())
    }

    /// `[user@]host[:port]`, for display.
    pub fn destination(&self) -> String {
        if self.transport == TransportKind::Local {
            return "(local)".to_string();
        }
        let mut out = String::new();
        if let Some(user) = &self.user {
            out.push_str(user);
            out.push('@');
        }
        out.push_str(self.host.as_deref().unwrap_or("?"));
        if let Some(port) = self.port {
            out.push_str(&format!(":{port}"));
        }
        out
    }

    pub fn os(&self) -> RemoteOs {
        self.os.unwrap_or_default()
    }

    pub fn workdir(&self) -> &str {
        self.workdir.as_deref().unwrap_or(DEFAULT_WORKDIR)
    }

    pub fn supports_platform(&self, platform: &str) -> bool {
        self.platforms.iter().any(|p| p == platform)
    }

    /// The local variables listed in `forward_env`, taken from `vars`. Empty
    /// values are skipped, like everywhere else in fastforge.
    pub fn forwarded_env<I>(&self, vars: I) -> BTreeMap<String, String>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        vars.into_iter()
            .filter(|(key, value)| {
                !value.is_empty()
                    && self
                        .forward_env
                        .iter()
                        .any(|pattern| match pattern.strip_suffix('*') {
                            Some(prefix) => key.starts_with(prefix),
                            None => key == pattern,
                        })
            })
            .collect()
    }

    fn validate(&self) -> Result<()> {
        if !is_valid_name(&self.name) {
            bail!(
                "Invalid host name `{}`: use letters, digits, `-`, `_` and `.`",
                self.name
            );
        }
        if self.name == "auto" {
            bail!("`auto` is reserved and cannot be used as a host name");
        }
        if self.transport == TransportKind::Ssh && self.host.as_deref().is_none_or(str::is_empty) {
            bail!("Host `{}` has no `host` address", self.name);
        }
        Ok(())
    }
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Contents of `hosts.yaml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HostsFile {
    #[serde(default)]
    pub hosts: Vec<HostConfig>,
}

impl HostsFile {
    /// `$FASTFORGE_HOSTS_FILE`, or `~/.fastforge/hosts.yaml`.
    pub fn default_path() -> Result<PathBuf> {
        if let Some(path) = std::env::var_os(HOSTS_FILE_ENV).filter(|p| !p.is_empty()) {
            return Ok(PathBuf::from(path));
        }
        let home = dirs::home_dir().context("could not determine the home directory")?;
        Ok(home.join(".fastforge").join("hosts.yaml"))
    }

    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_path()?)
    }

    /// Loads `path`, returning an empty file when it does not exist.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let file: HostsFile = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        for host in &file.hosts {
            host.validate()
                .with_context(|| format!("Invalid host in {}", path.display()))?;
        }
        Ok(file)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::default_path()?)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
    }

    pub fn get(&self, name: &str) -> Option<&HostConfig> {
        self.hosts.iter().find(|h| h.name == name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut HostConfig> {
        self.hosts.iter_mut().find(|h| h.name == name)
    }

    /// Like [`Self::get`], with an error that lists the known hosts.
    pub fn require(&self, name: &str) -> Result<&HostConfig> {
        self.get(name).ok_or_else(|| {
            let known: Vec<&str> = self.hosts.iter().map(|h| h.name.as_str()).collect();
            if known.is_empty() {
                anyhow!("Unknown host `{name}`: no hosts are configured (see `fastforge host add`)")
            } else {
                anyhow!("Unknown host `{name}`. Known hosts: {}", known.join(", "))
            }
        })
    }

    /// Adds `host`, or replaces the host of the same name when `replace`.
    pub fn add(&mut self, host: HostConfig, replace: bool) -> Result<()> {
        host.validate()?;
        match self.hosts.iter_mut().find(|h| h.name == host.name) {
            Some(existing) if replace => *existing = host,
            Some(_) => bail!(
                "Host `{}` already exists (use --force to replace it)",
                host.name
            ),
            None => self.hosts.push(host),
        }
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<HostConfig> {
        let index = self
            .hosts
            .iter()
            .position(|h| h.name == name)
            .ok_or_else(|| anyhow!("Unknown host `{name}`"))?;
        Ok(self.hosts.remove(index))
    }

    /// The first host declaring support for `platform`.
    pub fn find_for_platform(&self, platform: &str) -> Option<&HostConfig> {
        self.hosts.iter().find(|h| h.supports_platform(platform))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_destination_forms() {
        let mut host = HostConfig::new("a");
        host.set_destination("builder@mac-mini.local:2222").unwrap();
        assert_eq!(host.user.as_deref(), Some("builder"));
        assert_eq!(host.host.as_deref(), Some("mac-mini.local"));
        assert_eq!(host.port, Some(2222));
        assert_eq!(host.destination(), "builder@mac-mini.local:2222");

        let mut host = HostConfig::new("b");
        host.set_destination("192.168.1.20").unwrap();
        assert_eq!(host.user, None);
        assert_eq!(host.port, None);

        let mut host = HostConfig::new("c");
        host.set_destination("ci@[::1]:22").unwrap();
        assert_eq!(host.host.as_deref(), Some("::1"));
        assert_eq!(host.port, Some(22));

        assert!(HostConfig::new("d").set_destination("@host").is_err());
        assert!(HostConfig::new("e").set_destination("host:abc").is_err());
    }

    #[test]
    fn forwards_only_listed_variables() {
        let mut host = HostConfig::new("a");
        host.forward_env = vec!["APPLE_*".into(), "GITHUB_TOKEN".into()];
        let vars = vec![
            ("APPLE_ID".to_string(), "me".to_string()),
            ("APPLE_EMPTY".to_string(), String::new()),
            ("GITHUB_TOKEN".to_string(), "t".to_string()),
            ("GITHUB_TOKEN_2".to_string(), "x".to_string()),
            ("HOME".to_string(), "/home/me".to_string()),
        ];
        let forwarded = host.forwarded_env(vars);
        assert_eq!(
            forwarded.keys().collect::<Vec<_>>(),
            vec!["APPLE_ID", "GITHUB_TOKEN"]
        );
    }

    #[test]
    fn round_trips_and_validates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hosts.yaml");
        let mut file = HostsFile::default();
        let mut host = HostConfig::new("mac-mini");
        host.set_destination("builder@10.0.0.2").unwrap();
        host.platforms = vec!["ios".into(), "macos".into()];
        file.add(host.clone(), false).unwrap();
        assert!(file.add(host.clone(), false).is_err());
        file.add(host, true).unwrap();
        file.save_to(&path).unwrap();

        let loaded = HostsFile::load_from(&path).unwrap();
        assert_eq!(loaded, file);
        assert_eq!(loaded.find_for_platform("ios").unwrap().name, "mac-mini");
        assert!(loaded.find_for_platform("windows").is_none());
        assert!(loaded.require("nope").is_err());

        assert!(
            HostsFile::default()
                .add(HostConfig::new("x"), false)
                .is_err()
        );
        assert!(
            HostsFile::default()
                .add(HostConfig::new("bad name"), false)
                .is_err()
        );
        let mut local = HostConfig::new("auto");
        local.transport = TransportKind::Local;
        assert!(HostsFile::default().add(local, false).is_err());
    }

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = HostsFile::load_from(&dir.path().join("missing.yaml")).unwrap();
        assert!(file.hosts.is_empty());
    }
}
