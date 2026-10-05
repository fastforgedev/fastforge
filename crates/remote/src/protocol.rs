//! Messages exchanged between the local client and `fastforge remote-agent`.

use crate::manifest::Manifest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever the client/agent messages change incompatibly. The CLI
/// versions on both sides may differ as long as this matches.
pub const PROTOCOL_VERSION: u32 = 1;

/// Agent exit code: the workspace changed between reading its manifest and
/// syncing (another client synced meanwhile); the client retries.
pub const EXIT_WORKSPACE_CHANGED: i32 = 75;

/// Output of `remote-agent info`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub protocol: u32,
    pub version: String,
    pub os: String,
    pub arch: String,
    #[serde(default)]
    pub home: Option<String>,
    /// Tool name → resolved path (`None` when not on `PATH`).
    #[serde(default)]
    pub tools: BTreeMap<String, Option<String>>,
    /// Platforms this host can build for, judged from its OS and tools.
    #[serde(default)]
    pub platforms: Vec<String>,
}

/// Output of `remote-agent manifest`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestResponse {
    pub protocol: u32,
    pub manifest: Manifest,
}

/// First line written to `remote-agent exec`'s stdin. It is followed by
/// exactly `archive_len` bytes of `.tar.gz` holding the changed files; the
/// client then keeps stdin open until the agent exits, and closing it (e.g.
/// on Ctrl-C) stops the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    pub protocol: u32,
    /// Remote workspace directory (may start with `~/`).
    pub workspace: String,
    /// Digest of the manifest the client planned against. `None` skips the
    /// concurrent-change check.
    #[serde(default)]
    pub base_digest: Option<String>,
    /// Full manifest of the workspace after this sync.
    pub manifest: Manifest,
    #[serde(default)]
    pub deleted: Vec<String>,
    #[serde(default)]
    pub archive_len: u64,
    /// The fastforge command to run once synced; `None` only syncs.
    #[serde(default)]
    pub run: Option<RunSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSpec {
    pub id: String,
    /// Working directory relative to the workspace (`""` for its root).
    #[serde(default)]
    pub cwd: String,
    /// fastforge arguments, starting with the subcommand.
    pub argv: Vec<String>,
    /// Variables set verbatim (forwarded from the local environment).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Variables expanded on the remote host first (the host's `env`).
    #[serde(default)]
    pub env_templates: BTreeMap<String, String>,
    /// Append `--output <run output dir>`, so `fetch` can return exactly the
    /// files this run produced.
    #[serde(default)]
    pub append_output: bool,
    /// Don't run during `exec`: store the spec for a later
    /// `remote-agent attach`, which runs it on the client's terminal.
    #[serde(default)]
    pub interactive: bool,
}
