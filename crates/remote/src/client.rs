//! The local side: talks to `fastforge remote-agent` on a host.

use crate::agent::build_archive;
use crate::hosts::{HostConfig, RemoteOs};
use crate::manifest::{Manifest, fnv1a};
use crate::protocol::{
    AgentInfo, EXIT_WORKSPACE_CHANGED, ExecRequest, ManifestResponse, PROTOCOL_VERSION, RunSpec,
};
use crate::shell::{quote, quote_cmd, quote_cmd_path, quote_path};
use crate::transport::{self, Transport};
use anyhow::{Context, Result, anyhow, bail};
use flate2::read::GzDecoder;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Exit status of the agent when the remote `fastforge` binary is missing:
/// `sh` reports 127, `cmd.exe` 9009.
const EXIT_NOT_FOUND: i32 = 127;
const EXIT_NOT_FOUND_CMD: i32 = 9009;

/// A local project directory mapped onto a remote workspace.
#[derive(Debug, Clone)]
pub struct LocalProject {
    /// What gets synced: the enclosing git repository, or the working
    /// directory outside one (so `path:` dependencies inside the repository
    /// keep working).
    pub root: PathBuf,
    /// The working directory relative to `root`, `/`-separated (`""` at root).
    pub cwd: String,
}

impl LocalProject {
    pub fn detect(cwd: &Path) -> Result<Self> {
        let cwd = cwd
            .canonicalize()
            .with_context(|| format!("Failed to resolve {}", cwd.display()))?;
        let root = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(&cwd)
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
            .and_then(|p| p.canonicalize().ok())
            .filter(|root| cwd.starts_with(root))
            .unwrap_or_else(|| cwd.clone());
        Ok(Self::new(root, &cwd))
    }

    pub fn new(root: PathBuf, cwd: &Path) -> Self {
        let rel = cwd
            .strip_prefix(&root)
            .map(|rel| {
                rel.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default();
        Self { root, cwd: rel }
    }

    /// Converts a path relative to the working directory into one relative
    /// to `root`, or `None` when it lies outside the synced tree.
    pub fn key_for(&self, path: &Path) -> Option<String> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(&self.cwd).join(path)
        };
        let normalized = normalize(&absolute);
        let rel = normalized.strip_prefix(&self.root).ok()?;
        Some(
            rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    /// Stable remote directory name: `<root name>-<hash of the local path>`.
    pub fn workspace_name(&self) -> String {
        let base: String = self
            .root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "workspace".into())
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let hash = fnv1a(self.root.to_string_lossy().as_bytes());
        format!("{base}-{:012x}", hash & 0xffff_ffff_ffff)
    }
}

/// Lexically resolves `.` and `..` (the path may not exist yet).
fn normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Summary of a workspace sync.
#[derive(Debug, Default, Clone, Copy)]
pub struct SyncStats {
    pub changed: usize,
    pub deleted: usize,
    pub bytes: u64,
}

pub struct RemoteClient {
    pub(crate) host: HostConfig,
    pub(crate) transport: Box<dyn Transport>,
}

impl RemoteClient {
    pub fn new(host: HostConfig) -> Self {
        let transport = transport::for_host(&host);
        Self { host, transport }
    }

    pub fn host(&self) -> &HostConfig {
        &self.host
    }

    /// Remote workspace directory for `project`.
    pub fn workspace(&self, project: &LocalProject) -> String {
        format!(
            "{}/{}",
            self.host.workdir().trim_end_matches('/'),
            project.workspace_name()
        )
    }

    /// Shell command starting `fastforge remote-agent <args>` on the host.
    pub(crate) fn agent_script(&self, args: &[&str]) -> String {
        if self.host.os() == RemoteOs::Windows {
            let program = match &self.host.fastforge {
                Some(path) => quote_cmd_path(path),
                None => "fastforge".to_string(),
            };
            let args: Vec<String> = args.iter().map(|a| quote_cmd(a)).collect();
            return format!(
                "{program} --no-version-check remote-agent {}",
                args.join(" ")
            );
        }
        let locate = match &self.host.fastforge {
            Some(path) => format!("ff={}", quote_path(path)),
            None => format!(
                "ff=$(command -v fastforge) || {{ echo 'fastforge was not found on the PATH of {}; set `fastforge:` for this host' >&2; exit {EXIT_NOT_FOUND}; }}",
                self.host.name.replace('\'', "")
            ),
        };
        let args: Vec<String> = args.iter().map(|a| quote(a)).collect();
        let script = format!(
            "{locate}; exec \"$ff\" --no-version-check remote-agent {}",
            args.join(" ")
        );
        self.transport.login_script(&script)
    }

    fn agent_command(&self, args: &[&str]) -> Command {
        self.transport.command(&self.agent_script(args), false)
    }

    /// Runs an empty command on the host to check the connection.
    pub fn ping(&self) -> Result<()> {
        let script = match self.host.os() {
            RemoteOs::Unix => "true",
            RemoteOs::Windows => "exit 0",
        };
        let status = self
            .transport
            .command(script, false)
            .stdin(Stdio::null())
            .status()
            .context("Failed to start the transport")?;
        if !status.success() {
            bail!(
                "Could not connect to `{}` ({})",
                self.host.name,
                self.host.destination()
            );
        }
        Ok(())
    }

    fn capture_json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> Result<T> {
        let output = self
            .agent_command(args)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .context("Failed to start the transport")?;
        if !output.status.success() {
            bail!(
                "`fastforge remote-agent {}` failed on `{}` ({})",
                args.first().copied().unwrap_or_default(),
                self.host.name,
                output.status
            );
        }
        serde_json::from_slice(&output.stdout).with_context(|| {
            format!(
                "Unexpected response from `{}`; is its fastforge recent enough to support remote hosts?",
                self.host.name
            )
        })
    }

    /// Asks the host's default shell what it is: `cmd.exe` expands `%OS%` to
    /// `Windows_NT`, a POSIX shell prints it unchanged.
    pub fn detect_os(&self) -> Result<RemoteOs> {
        let output = self
            .transport
            .command("echo %OS%", false)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .context("Failed to start the transport")?;
        if !output.status.success() {
            bail!(
                "Could not connect to `{}` ({})",
                self.host.name,
                self.host.destination()
            );
        }
        Ok(
            if String::from_utf8_lossy(&output.stdout).contains("Windows_NT") {
                RemoteOs::Windows
            } else {
                RemoteOs::Unix
            },
        )
    }

    /// Fills in the host's OS when it isn't configured.
    pub fn ensure_os(&mut self) -> Result<RemoteOs> {
        if let Some(os) = self.host.os {
            return Ok(os);
        }
        let os = self.detect_os()?;
        self.host.os = Some(os);
        Ok(os)
    }

    pub fn info(&self) -> Result<AgentInfo> {
        let info: AgentInfo = self.capture_json(&["info"])?;
        check_protocol(&self.host.name, info.protocol)?;
        Ok(info)
    }

    fn remote_manifest(&self, workspace: &str) -> Result<Manifest> {
        let response: ManifestResponse =
            self.capture_json(&["manifest", "--workspace", workspace])?;
        check_protocol(&self.host.name, response.protocol)?;
        Ok(response.manifest)
    }

    /// Syncs `project` into `workspace` and, when `run` is given, runs it
    /// there, streaming its output. Returns the remote exit code.
    pub fn sync_and_run(
        &self,
        project: &LocalProject,
        workspace: &str,
        excludes: &[String],
        run: Option<RunSpec>,
    ) -> Result<(i32, SyncStats)> {
        self.sync_and_run_with(project, workspace, excludes, run, true)
    }

    /// Syncs `project` without announcing it; for repeated syncs during an
    /// interactive run.
    pub fn resync(
        &self,
        project: &LocalProject,
        workspace: &str,
        excludes: &[String],
    ) -> Result<SyncStats> {
        let (code, stats) = self.sync_and_run_with(project, workspace, excludes, None, false)?;
        if code != 0 {
            bail!("Syncing to `{}` failed (exit code {code})", self.host.name);
        }
        Ok(stats)
    }

    fn sync_and_run_with(
        &self,
        project: &LocalProject,
        workspace: &str,
        excludes: &[String],
        run: Option<RunSpec>,
        announce: bool,
    ) -> Result<(i32, SyncStats)> {
        let local = Manifest::scan(&project.root, excludes)?;
        for attempt in 0..3 {
            let remote = self.remote_manifest(workspace)?;
            let plan = local.plan_from(&remote);
            let (mut archive, archive_len) = if plan.changed.is_empty() {
                (None, 0)
            } else {
                let (file, len) = build_archive(&project.root, &plan.changed)?;
                (Some(file), len)
            };
            let stats = SyncStats {
                changed: plan.changed.len(),
                deleted: plan.deleted.len(),
                bytes: archive_len,
            };
            if attempt == 0 && announce {
                eprintln!(
                    "Syncing {} to {}:{} ({} changed, {} deleted, {})",
                    project.root.display(),
                    self.host.name,
                    workspace,
                    stats.changed,
                    stats.deleted,
                    human_bytes(stats.bytes)
                );
            }
            let request = ExecRequest {
                protocol: PROTOCOL_VERSION,
                workspace: workspace.to_string(),
                base_digest: Some(remote.digest()),
                manifest: local.clone(),
                deleted: plan.deleted,
                archive_len,
                run: run.clone(),
            };

            let mut child = self
                .agent_command(&["exec"])
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .context("Failed to start the transport")?;
            // Keep stdin open until the agent exits: closing it stops the run.
            let mut stdin = child.stdin.take().expect("piped stdin");
            let sent = (|| -> std::io::Result<()> {
                serde_json::to_writer(&mut stdin, &request)?;
                stdin.write_all(b"\n")?;
                if let Some(file) = archive.as_mut() {
                    std::io::copy(file, &mut stdin)?;
                }
                stdin.flush()
            })();
            let status = child.wait()?;
            drop(stdin);
            if let Err(error) = sent
                && status.success()
            {
                return Err(error).context("Failed to send the workspace");
            }
            match status.code() {
                Some(EXIT_WORKSPACE_CHANGED) => continue,
                Some(EXIT_NOT_FOUND | EXIT_NOT_FOUND_CMD) => bail!(
                    "fastforge was not found on `{}`; install it there or set `fastforge:` for this host",
                    self.host.name
                ),
                Some(code) => return Ok((code, stats)),
                None => bail!("The connection to `{}` was interrupted", self.host.name),
            }
        }
        Err(anyhow!(
            "The workspace on `{}` kept changing during sync; is another client syncing the same project?",
            self.host.name
        ))
    }

    /// Downloads the output of `run_id` into `dest`, returning the files.
    pub fn fetch(&self, workspace: &str, run_id: &str, dest: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dest)
            .with_context(|| format!("Failed to create {}", dest.display()))?;
        let mut child = self
            .agent_command(&["fetch", "--workspace", workspace, "--run", run_id])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to start the transport")?;
        let stdout = child.stdout.take().expect("piped stdout");
        let files = unpack_into(stdout, dest);
        let status = child.wait()?;
        if !status.success() {
            bail!(
                "Failed to fetch the artifacts from `{}` ({status})",
                self.host.name
            );
        }
        files
    }

    /// Runs `command` in the workspace through the login shell (`cmd.exe` on
    /// Windows).
    pub fn shell(&self, workspace: &str, cwd: &str, command: &str, tty: bool) -> Result<i32> {
        if self.host.os() == RemoteOs::Windows {
            let dir = if cwd.is_empty() {
                workspace.to_string()
            } else {
                format!("{workspace}/{cwd}")
            };
            let script = format!("cd /d {} && {command}", quote_cmd_path(&dir));
            let status = self
                .transport
                .command(&script, tty)
                .status()
                .context("Failed to start the transport")?;
            return Ok(status.code().unwrap_or(1));
        }
        let mut dir = quote_path(workspace);
        if !cwd.is_empty() {
            dir.push('/');
            dir.push_str(&quote(cwd));
        }
        let script = self
            .transport
            .login_script(&format!("cd {dir} && {command}"));
        let status = self
            .transport
            .command(&script, tty)
            .status()
            .context("Failed to start the transport")?;
        Ok(status.code().unwrap_or(1))
    }
}

fn unpack_into<R: Read>(input: R, dest: &Path) -> Result<Vec<PathBuf>> {
    let mut archive = tar::Archive::new(GzDecoder::new(input));
    archive.set_preserve_mtime(true);
    archive.set_overwrite(true);
    let mut files = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let is_file = entry.header().entry_type().is_file();
        let path = entry.path()?.into_owned();
        if !entry.unpack_in(dest)? {
            bail!(
                "Refusing to extract {} outside {}",
                path.display(),
                dest.display()
            );
        }
        if is_file {
            files.push(dest.join(path));
        }
    }
    Ok(files)
}

fn check_protocol(host: &str, protocol: u32) -> Result<()> {
    if protocol != PROTOCOL_VERSION {
        bail!(
            "fastforge on `{host}` speaks remote protocol {protocol}, this one speaks {PROTOCOL_VERSION}; install matching versions"
        );
    }
    Ok(())
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// A run id unique enough for one host: time plus process id.
pub fn new_run_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{:x}", nanos, std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_paths_into_the_project() {
        let project = LocalProject::new(PathBuf::from("/repo"), Path::new("/repo/apps/mobile"));
        assert_eq!(project.cwd, "apps/mobile");
        assert_eq!(
            project
                .key_for(Path::new("ios/ExportOptions.plist"))
                .as_deref(),
            Some("apps/mobile/ios/ExportOptions.plist")
        );
        assert_eq!(
            project.key_for(Path::new("../shared/x")).as_deref(),
            Some("apps/shared/x")
        );
        assert_eq!(project.key_for(Path::new("/repo/a")).as_deref(), Some("a"));
        assert_eq!(project.key_for(Path::new("/etc/passwd")), None);
        assert_eq!(project.key_for(Path::new("../../../x")), None);
    }

    #[test]
    fn workspace_name_is_stable_and_safe() {
        let project = LocalProject::new(
            PathBuf::from("/Users/me/My App"),
            Path::new("/Users/me/My App"),
        );
        let name = project.workspace_name();
        assert!(name.starts_with("My_App-"));
        assert_eq!(name, project.workspace_name());
        let other = LocalProject::new(PathBuf::from("/tmp/My App"), Path::new("/tmp/My App"));
        assert_ne!(name, other.workspace_name());
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(human_bytes(12), "12 B");
        assert_eq!(human_bytes(2048), "2.0 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MB");
    }
}
