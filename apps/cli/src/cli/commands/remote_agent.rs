//! `fastforge remote-agent`: the hidden entry point a client starts on a
//! remote host. Not meant to be run by hand.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use fastforge_remote::agent;
use std::io::Write;

#[derive(Args)]
pub struct RemoteAgentArgs {
    #[command(subcommand)]
    pub command: RemoteAgentCommands,
}

#[derive(Subcommand)]
pub enum RemoteAgentCommands {
    /// Print the agent's version, protocol, OS and tools as JSON
    Info,
    /// Print the workspace manifest as JSON
    Manifest {
        #[arg(long)]
        workspace: String,
    },
    /// Read an exec request from stdin, sync the workspace and run it
    Exec,
    /// Run an interactive run stored by `exec` on this terminal
    Attach {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        run: String,
    },
    /// Write a run's output directory to stdout as .tar.gz
    Fetch {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        run: String,
    },
    /// Download what remote windows need, if not done yet
    RemoteWindowSetup,
    /// Start the Host of a remote window in the desktop session for one
    /// Client, print where it listens as JSON, and stop it when stdin closes
    RemoteWindow {
        /// Fingerprint of the Client allowed to connect.
        #[arg(long)]
        client: String,
        /// Windows to move onto the virtual display (title or process name).
        #[arg(long)]
        filter: String,
    },
}

pub async fn execute(args: &RemoteAgentArgs) -> Result<()> {
    match &args.command {
        RemoteAgentCommands::Info => print_json(&agent::info(env!("FASTFORGE_BUILD_VERSION"))),
        RemoteAgentCommands::Manifest { workspace } => print_json(&agent::manifest(workspace)),
        RemoteAgentCommands::Exec => {
            let program = std::env::current_exe().context("Failed to locate fastforge")?;
            let code = tokio::task::spawn_blocking(move || agent::exec(std::io::stdin(), &program))
                .await??;
            std::io::stdout().flush()?;
            std::process::exit(code);
        }
        RemoteAgentCommands::Attach { workspace, run } => {
            let program = std::env::current_exe().context("Failed to locate fastforge")?;
            // Ctrl-C on the remote terminal is for the run; outlive it.
            let guard = tokio::spawn(async { while tokio::signal::ctrl_c().await.is_ok() {} });
            let (workspace, run) = (workspace.clone(), run.clone());
            let code =
                tokio::task::spawn_blocking(move || agent::attach(&workspace, &run, &program))
                    .await??;
            guard.abort();
            std::process::exit(code);
        }
        RemoteAgentCommands::Fetch { workspace, run } => {
            agent::fetch(workspace, run, std::io::stdout().lock())
        }
        RemoteAgentCommands::RemoteWindowSetup => {
            super::remote_window::ensure_dazzdesk().await.map(|_| ())
        }
        RemoteAgentCommands::RemoteWindow { client, filter } => {
            let (client, filter) = (client.clone(), filter.clone());
            tokio::task::spawn_blocking(move || super::remote_window::serve_host(&client, &filter))
                .await?
        }
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
