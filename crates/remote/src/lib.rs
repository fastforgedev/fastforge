//! Remote host support: run fastforge commands on another machine.
//!
//! The local CLI syncs the project to a workspace on the host, starts
//! `fastforge remote-agent exec` there (fastforge itself does the syncing,
//! locking and running), streams its output, and fetches the artifacts back.

pub mod agent;
pub mod client;
pub mod hosts;
pub mod interactive;
pub mod manifest;
pub mod protocol;
pub mod shell;
pub mod transport;

pub use client::{LocalProject, RemoteClient, SyncStats, new_run_id};
pub use hosts::{HostConfig, HostsFile, RemoteOs, TransportKind};
pub use protocol::{AgentInfo, PROTOCOL_VERSION, RunSpec};
