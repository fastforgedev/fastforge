//! `fastforge host`: manage the remote hosts in `~/.fastforge/hosts.yaml`.

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Subcommand};
use fastforge_remote::{
    HostConfig, HostsFile, LocalProject, PROTOCOL_VERSION, RemoteClient, RemoteOs, TransportKind,
};
use std::io::IsTerminal;

use crate::utils::{bright_green, yellow};

#[derive(Args)]
pub struct HostArgs {
    #[command(subcommand)]
    pub command: HostCommands,
}

#[derive(Subcommand)]
pub enum HostCommands {
    /// Add (or with --force, replace) a remote host
    Add(HostAddArgs),
    /// List the configured remote hosts
    List,
    /// Remove a remote host
    Remove {
        /// Host name
        name: String,
    },
    /// Check the connection, the remote fastforge and its tools
    Doctor {
        /// Host name
        name: String,
    },
    /// Sync the current project to a host and run a shell command there
    Exec {
        /// Host name
        name: String,
        /// Run in the remote workspace without syncing first
        #[arg(long = "no-sync")]
        no_sync: bool,
        /// The command (joined with spaces and run by the remote login shell)
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
}

#[derive(Args)]
pub struct HostAddArgs {
    /// Host name used with `--host` (letters, digits, `-`, `_`, `.`)
    pub name: String,
    /// SSH destination: `[user@]host[:port]` or a `~/.ssh/config` alias
    pub destination: Option<String>,
    #[arg(long = "transport", value_name = "ssh|local", default_value = "ssh")]
    pub transport: String,
    /// Host OS family (default: detected by `host doctor`)
    #[arg(long = "os", value_name = "unix|windows")]
    pub os: Option<String>,
    /// Private key passed to `ssh -i`
    #[arg(long = "identity-file", value_name = "PATH")]
    pub identity_file: Option<String>,
    /// Remote directory for synced workspaces (default ~/.fastforge/remote/workspaces)
    #[arg(long = "workdir", value_name = "DIR")]
    pub workdir: Option<String>,
    /// Path of fastforge on the host (default: found on its PATH)
    #[arg(long = "fastforge", value_name = "PATH")]
    pub fastforge: Option<String>,
    /// Comma-separated platforms the host builds (default: detected by `host doctor`)
    #[arg(long = "platforms", value_name = "ios,macos,...")]
    pub platforms: Option<String>,
    /// Comma-separated local variables to forward; `PREFIX_*` matches a prefix
    #[arg(long = "forward-env", value_name = "NAME,PREFIX_*")]
    pub forward_env: Option<String>,
    /// Variable set for remote runs (expanded on the host); may be repeated
    #[arg(long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    /// Replace an existing host of the same name
    #[arg(long = "force")]
    pub force: bool,
}

fn split_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

impl HostAddArgs {
    fn to_config(&self) -> Result<HostConfig> {
        let mut host = HostConfig::new(&self.name);
        host.transport = match self.transport.as_str() {
            "ssh" => TransportKind::Ssh,
            "local" => TransportKind::Local,
            other => bail!("Unknown transport `{other}` (expected ssh or local)"),
        };
        match (&self.destination, host.transport) {
            (Some(destination), _) => host.set_destination(destination)?,
            (None, TransportKind::Ssh) => bail!("An SSH host needs a destination"),
            (None, TransportKind::Local) => {}
        }
        host.os = match self.os.as_deref() {
            None => None,
            Some("unix") => Some(RemoteOs::Unix),
            Some("windows") => Some(RemoteOs::Windows),
            Some(other) => bail!("Unknown OS `{other}` (expected unix or windows)"),
        };
        host.identity_file = self.identity_file.clone();
        host.workdir = self.workdir.clone();
        host.fastforge = self.fastforge.clone();
        host.platforms = split_list(self.platforms.as_deref());
        host.forward_env = split_list(self.forward_env.as_deref());
        for item in &self.env {
            let (key, value) = item
                .split_once('=')
                .ok_or_else(|| anyhow!("Invalid --env `{item}`, expected KEY=VALUE"))?;
            host.env.insert(key.to_string(), value.to_string());
        }
        Ok(host)
    }
}

pub async fn execute(args: &HostArgs) -> Result<()> {
    match &args.command {
        HostCommands::Add(add) => {
            let mut hosts = HostsFile::load()?;
            let host = add.to_config()?;
            let name = host.name.clone();
            hosts.add(host, add.force)?;
            hosts.save()?;
            println!(
                "Added host `{name}` to {}. Run `fastforge host doctor {name}` to check it.",
                HostsFile::default_path()?.display()
            );
            Ok(())
        }
        HostCommands::List => list(),
        HostCommands::Remove { name } => {
            let mut hosts = HostsFile::load()?;
            hosts.remove(name)?;
            hosts.save()?;
            println!("Removed host `{name}`.");
            Ok(())
        }
        HostCommands::Doctor { name } => doctor(name),
        HostCommands::Exec {
            name,
            no_sync,
            command,
        } => exec(name, *no_sync, command),
    }
}

fn list() -> Result<()> {
    let hosts = HostsFile::load()?;
    if hosts.hosts.is_empty() {
        println!(
            "No remote hosts configured. Add one with `fastforge host add <name> <user@host>`."
        );
        return Ok(());
    }
    let rows: Vec<[String; 4]> = hosts
        .hosts
        .iter()
        .map(|h| {
            [
                h.name.clone(),
                h.destination(),
                if h.platforms.is_empty() {
                    "-".to_string()
                } else {
                    h.platforms.join(",")
                },
                h.workdir().to_string(),
            ]
        })
        .collect();
    let headers = ["NAME", "DESTINATION", "PLATFORMS", "WORKDIR"];
    let widths: Vec<usize> = (0..4)
        .map(|i| {
            rows.iter()
                .map(|r| r[i].len())
                .chain([headers[i].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: [&str; 4]| {
        let padded: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!("{c:<w$}"))
            .collect();
        println!("{}", padded.join("  ").trim_end());
    };
    line(headers);
    for row in &rows {
        line([&row[0], &row[1], &row[2], &row[3]]);
    }
    Ok(())
}

fn doctor(name: &str) -> Result<()> {
    let mut hosts = HostsFile::load()?;
    let host = hosts.require(name)?.clone();
    let mut client = RemoteClient::new(host.clone());

    println!(
        "Host {} ({}, {})",
        host.name,
        host.destination(),
        host.transport.as_str()
    );
    let os = client.ensure_os()?;
    client.ping()?;
    println!("  {} connection ({} shell)", bright_green("✓"), os.as_str());
    if host.os.is_none() {
        if let Some(saved) = hosts.get_mut(name) {
            saved.os = Some(os);
        }
        hosts.save()?;
    }

    let info = client.info()?;
    println!(
        "  {} fastforge {} (remote protocol {}) on {} {}",
        bright_green("✓"),
        info.version,
        info.protocol,
        info.os,
        info.arch
    );
    let local_version = env!("FASTFORGE_BUILD_VERSION");
    if info.version != local_version {
        println!(
            "  {}",
            yellow(&format!(
                "! local fastforge is {local_version}; versions may differ while the protocol ({PROTOCOL_VERSION}) matches"
            ))
        );
    }
    for (tool, path) in &info.tools {
        match path {
            Some(path) => println!("  {} {tool}: {path}", bright_green("✓")),
            None => println!("  {} {tool}: not found", yellow("-")),
        }
    }
    println!("  detected platforms: {}", info.platforms.join(", "));

    if host.platforms.is_empty() && !info.platforms.is_empty() {
        if let Some(saved) = hosts.get_mut(name) {
            saved.platforms = info.platforms.clone();
        }
        hosts.save()?;
        println!(
            "  saved platforms to {}",
            HostsFile::default_path()?.display()
        );
    } else if !host.platforms.is_empty() {
        let missing: Vec<&String> = host
            .platforms
            .iter()
            .filter(|p| !info.platforms.contains(p))
            .collect();
        if !missing.is_empty() {
            println!(
                "  {}",
                yellow(&format!(
                    "! configured platforms not detected on the host: {}",
                    missing
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            );
        }
    }
    Ok(())
}

fn exec(name: &str, no_sync: bool, command: &[String]) -> Result<()> {
    let hosts = HostsFile::load()?;
    let host = hosts.require(name)?.clone();
    let mut client = RemoteClient::new(host.clone());
    client.ensure_os()?;
    let cwd = std::env::current_dir()?;
    let project = LocalProject::detect(&cwd)?;
    let workspace = client.workspace(&project);
    if !no_sync {
        let (code, _) = client
            .sync_and_run(
                &project,
                &workspace,
                &super::remote::sync_excludes(&project, None)?,
                None,
            )
            .context("Failed to sync the workspace")?;
        if code != 0 {
            bail!("Syncing to `{name}` failed (exit code {code})");
        }
    }
    let tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let code = client.shell(&workspace, &project.cwd, &command.join(" "), tty)?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        args: HostAddArgs,
    }

    #[test]
    fn builds_a_host_from_arguments() {
        let cli = Cli::parse_from([
            "x",
            "mac",
            "builder@10.0.0.2:2200",
            "--platforms",
            "ios, macos",
            "--forward-env",
            "APPLE_*,GITHUB_TOKEN",
            "--env",
            "PATH=~/flutter/bin:$PATH",
        ]);
        let host = cli.args.to_config().unwrap();
        assert_eq!(host.user.as_deref(), Some("builder"));
        assert_eq!(host.port, Some(2200));
        assert_eq!(host.platforms, vec!["ios", "macos"]);
        assert_eq!(host.forward_env, vec!["APPLE_*", "GITHUB_TOKEN"]);
        assert_eq!(host.env["PATH"], "~/flutter/bin:$PATH");

        let cli = Cli::parse_from(["x", "mac"]);
        assert!(cli.args.to_config().is_err());
        let cli = Cli::parse_from(["x", "here", "--transport", "local"]);
        assert_eq!(
            cli.args.to_config().unwrap().transport,
            TransportKind::Local
        );
    }
}
