//! The remote side: `fastforge remote-agent <info|manifest|exec|fetch>`.
//!
//! The agent is fastforge itself, so syncing, locking and running need no
//! tools on the remote host beyond a shell to start it.

use crate::manifest::{Manifest, validate_key};
use crate::protocol::{
    AgentInfo, EXIT_WORKSPACE_CHANGED, ExecRequest, ManifestResponse, PROTOCOL_VERSION, RunSpec,
};
use crate::shell::{expand_env_templates, expand_home};
use anyhow::{Context, Result, anyhow, bail};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

/// Run directories older than this are removed at the start of each run.
const STALE_RUN_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Tools reported by `info`.
const TOOLS: &[&str] = &["flutter", "dart", "xcodebuild", "java", "hvigorw"];

fn state_dir(workspace: &Path) -> PathBuf {
    workspace.join(".fastforge").join("remote")
}

fn manifest_path(workspace: &Path) -> PathBuf {
    state_dir(workspace).join("manifest.json")
}

fn run_dir(workspace: &Path, run_id: &str) -> PathBuf {
    state_dir(workspace).join("runs").join(run_id)
}

/// Output directory of a run, inside the workspace.
pub fn run_output_dir(workspace: &Path, run_id: &str) -> PathBuf {
    run_dir(workspace, run_id).join("out")
}

pub fn resolve_workspace(workspace: &str) -> PathBuf {
    // The client always writes `/`; use the native separator for display.
    let native = workspace.replace('/', std::path::MAIN_SEPARATOR_STR);
    let native = native.replacen(&format!("~{}", std::path::MAIN_SEPARATOR), "~/", 1);
    expand_home(&native, dirs::home_dir().as_deref())
}

fn validate_run_id(id: &str) -> Result<()> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("Invalid run id `{id}`");
    }
    Ok(())
}

pub fn info(version: &str) -> AgentInfo {
    let tools: BTreeMap<String, Option<String>> = TOOLS
        .iter()
        .map(|tool| {
            (
                tool.to_string(),
                which(tool).map(|p| p.to_string_lossy().into_owned()),
            )
        })
        .collect();
    let os = std::env::consts::OS;
    let mut platforms: Vec<&str> = vec!["android", "web"];
    match os {
        "macos" => {
            platforms.push("macos");
            if tools.get("xcodebuild").is_some_and(Option::is_some) {
                platforms.push("ios");
            }
        }
        "linux" => platforms.push("linux"),
        "windows" => platforms.push("windows"),
        _ => {}
    }
    if tools.get("hvigorw").is_some_and(Option::is_some) {
        platforms.push("ohos");
    }
    platforms.sort_unstable();
    AgentInfo {
        protocol: PROTOCOL_VERSION,
        version: version.to_string(),
        os: os.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        home: dirs::home_dir().map(|h| h.to_string_lossy().into_owned()),
        tools,
        platforms: platforms.into_iter().map(String::from).collect(),
    }
}

/// `path` with entries written as `~/...` expanded. Shell profiles sometimes
/// quote the tilde (`PATH="~/flutter/bin:$PATH"`); bash still finds commands
/// there, but no other program does.
pub fn expand_path_entries(path: &std::ffi::OsStr, home: Option<&Path>) -> std::ffi::OsString {
    let entries: Vec<PathBuf> = std::env::split_paths(path)
        .map(|entry| match entry.to_str() {
            Some(text) if text == "~" || text.starts_with("~/") => expand_home(text, home),
            _ => entry,
        })
        .collect();
    std::env::join_paths(&entries).unwrap_or_else(|_| path.to_os_string())
}

/// This process's `PATH`, with `~` entries expanded.
fn effective_path() -> Option<std::ffi::OsString> {
    let path = std::env::var_os("PATH")?;
    Some(expand_path_entries(&path, dirs::home_dir().as_deref()))
}

/// Finds `name` on this process's `PATH` (with `~` entries expanded).
pub fn which(name: &str) -> Option<PathBuf> {
    let path = effective_path()?;
    let suffixes: &[&str] = if cfg!(windows) {
        &[".exe", ".bat", ".cmd", ""]
    } else {
        &[""]
    };
    std::env::split_paths(&path).find_map(|dir| {
        suffixes.iter().find_map(|suffix| {
            let candidate = dir.join(format!("{name}{suffix}"));
            candidate.is_file().then_some(candidate)
        })
    })
}

/// The recorded manifest, minus entries whose files changed on disk since.
fn current_manifest(workspace: &Path) -> Manifest {
    fs::read(manifest_path(workspace))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Manifest>(&bytes).ok())
        .map(|m| m.verified(workspace))
        .unwrap_or_default()
}

pub fn manifest(workspace: &str) -> ManifestResponse {
    ManifestResponse {
        protocol: PROTOCOL_VERSION,
        manifest: current_manifest(&resolve_workspace(workspace)),
    }
}

/// Handles `exec`: reads the request from `input`, syncs the workspace under
/// its lock and runs the requested fastforge command with `program`. Returns
/// the exit code to exit with.
pub fn exec<R: Read + Send + 'static>(input: R, program: &Path) -> Result<i32> {
    let mut reader = BufReader::new(input);
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .context("Failed to read the exec request")?;
    let request: ExecRequest =
        serde_json::from_str(header.trim()).context("Invalid exec request")?;
    if request.protocol != PROTOCOL_VERSION {
        bail!(
            "Protocol mismatch: client speaks {}, this fastforge speaks {}. Install matching fastforge versions.",
            request.protocol,
            PROTOCOL_VERSION
        );
    }

    let workspace = resolve_workspace(&request.workspace);
    fs::create_dir_all(state_dir(&workspace))
        .with_context(|| format!("Failed to create {}", workspace.display()))?;
    // A run waits for any other run in this workspace before syncing, so it
    // doesn't change files under a build in progress. A plain sync (e.g.
    // before a hot reload of the run itself) doesn't wait.
    let run_lock = match &request.run {
        Some(run) if !run.interactive => Some(lock_workspace(&workspace, RUN_LOCK)?),
        // `attach` takes the run lock; only wait for it to be free here.
        Some(_) => {
            drop(lock_workspace(&workspace, RUN_LOCK)?);
            None
        }
        None => None,
    };
    let sync_lock = lock_workspace(&workspace, SYNC_LOCK)?;
    if let Some(base) = &request.base_digest
        && current_manifest(&workspace).digest() != *base
    {
        eprintln!("The remote workspace changed while syncing; retrying.");
        return Ok(EXIT_WORKSPACE_CHANGED);
    }
    apply_sync(&workspace, &request, &mut reader)?;
    drop(sync_lock);

    let Some(run) = request.run else {
        return Ok(0);
    };
    if run.interactive {
        store_interactive_spec(&workspace, &run)?;
        return Ok(0);
    }
    let code = run_command(&workspace, &run, program, reader)?;
    drop(run_lock);
    Ok(code)
}

/// Held while files are synced into the workspace.
const SYNC_LOCK: &str = "lock";
/// Held for a whole run (`package`, interactive `run`), so runs in one
/// workspace don't build over each other.
const RUN_LOCK: &str = "run.lock";

/// Locks `name` in the workspace's state directory, waiting if another
/// process holds it. Released when the file is dropped or the process exits.
fn lock_workspace(workspace: &Path, name: &str) -> Result<File> {
    let path = state_dir(workspace).join(name);
    let file = File::create(&path).with_context(|| format!("Failed to open {}", path.display()))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => {
            let what = if name == RUN_LOCK { "run" } else { "sync" };
            eprintln!("Waiting for another fastforge {what} in this workspace to finish...");
            file.lock()
                .with_context(|| format!("Failed to lock {}", path.display()))?;
        }
        Err(fs::TryLockError::Error(error)) => {
            return Err(error).with_context(|| format!("Failed to lock {}", path.display()));
        }
    }
    Ok(file)
}

fn apply_sync<R: Read>(workspace: &Path, request: &ExecRequest, reader: &mut R) -> Result<()> {
    for key in &request.deleted {
        validate_key(key)?;
        let path = workspace.join(key);
        match fs::symlink_metadata(&path) {
            Ok(meta) if !meta.is_dir() => {
                fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove {}", path.display()))?;
            }
            _ => {}
        }
    }

    if request.archive_len > 0 {
        // Spool the archive first so the rest of stdin stays aligned no matter
        // how much of the gzip trailer the decoder consumes.
        let mut spool = tempfile::tempfile().context("Failed to create a temporary file")?;
        let copied = std::io::copy(&mut reader.take(request.archive_len), &mut spool)?;
        if copied != request.archive_len {
            bail!(
                "Truncated sync archive ({copied} of {} bytes)",
                request.archive_len
            );
        }
        use std::io::Seek;
        spool.rewind()?;
        let mut archive = tar::Archive::new(GzDecoder::new(spool));
        archive.set_preserve_mtime(true);
        archive.set_preserve_permissions(true);
        archive.set_overwrite(true);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let key = entry.path()?.to_string_lossy().replace('\\', "/");
            validate_key(&key)?;
            // Never write through a symlink left at the destination.
            let target = workspace.join(&key);
            if fs::symlink_metadata(&target).is_ok_and(|m| !m.is_dir()) {
                fs::remove_file(&target)?;
            }
            let is_link = entry.header().entry_type().is_symlink();
            match entry.unpack_in(workspace) {
                Ok(_) => {}
                // Creating symlinks on Windows needs Developer Mode or admin
                // rights; skip them rather than failing the whole sync.
                Err(error) if cfg!(windows) && is_link => {
                    eprintln!("Warning: skipped symbolic link {key}: {error}");
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Failed to extract {key}"));
                }
            }
        }
    }

    let path = manifest_path(workspace);
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec(&request.manifest)?)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

fn purge_stale_runs(workspace: &Path) {
    let Ok(entries) = fs::read_dir(state_dir(workspace).join("runs")) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age > STALE_RUN_AGE);
        if stale {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

fn spec_path(workspace: &Path, run_id: &str) -> PathBuf {
    run_dir(workspace, run_id).join("spec.json")
}

/// Saves an interactive run for `attach`. The spec holds forwarded secrets,
/// so it is private to the user and removed as soon as it is read.
fn store_interactive_spec(workspace: &Path, run: &RunSpec) -> Result<()> {
    validate_run_id(&run.id)?;
    purge_stale_runs(workspace);
    fs::create_dir_all(run_dir(workspace, &run.id))?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let path = spec_path(workspace, &run.id);
    let mut file = options
        .open(&path)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    file.write_all(&serde_json::to_vec(run)?)?;
    Ok(())
}

/// Handles `attach`: runs the interactive run stored by `exec` with this
/// process's stdio, which the client connects to its terminal.
pub fn attach(workspace: &str, run_id: &str, program: &Path) -> Result<i32> {
    validate_run_id(run_id)?;
    let workspace = resolve_workspace(workspace);
    let path = spec_path(&workspace, run_id);
    let bytes = fs::read(&path).with_context(|| format!("No pending run `{run_id}`"))?;
    let _ = fs::remove_file(&path);
    let run: RunSpec = serde_json::from_slice(&bytes).context("Invalid run spec")?;
    let _run_lock = lock_workspace(&workspace, RUN_LOCK)?;

    let mut command = child_command(&workspace, &run, program)?;
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    // On a terminal, a disconnect hangs up the whole session. Without one,
    // relay stdin and stop the run when the client goes away.
    let interactive = std::io::IsTerminal::is_terminal(&std::io::stdin());
    if interactive {
        command.stdin(Stdio::inherit());
    } else {
        command.stdin(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("Failed to start {}", program.display()))?;
    if let Some(mut child_stdin) = child.stdin.take() {
        let pid = child.id();
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut buf = [0u8; 4096];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if child_stdin.write_all(&buf[..n]).is_err() {
                            return;
                        }
                        let _ = child_stdin.flush();
                    }
                }
            }
            drop(child_stdin);
            terminate_group(pid);
        });
    }
    let status = child.wait()?;
    let _ = fs::remove_dir_all(run_dir(&workspace, run_id));
    Ok(status.code().unwrap_or(1))
}

/// The fastforge child process for `run`, with its working directory and
/// environment set.
fn child_command(workspace: &Path, run: &RunSpec, program: &Path) -> Result<Command> {
    validate_run_id(&run.id)?;
    let cwd = if run.cwd.is_empty() || run.cwd == "." {
        workspace.to_path_buf()
    } else {
        validate_key(&run.cwd)?;
        workspace.join(&run.cwd)
    };
    let path = effective_path();
    let base = |name: &str| {
        run.env.get(name).cloned().or_else(|| {
            if name == "PATH" {
                path.as_ref().map(|p| p.to_string_lossy().into_owned())
            } else {
                std::env::var(name).ok()
            }
        })
    };
    let templates = expand_env_templates(&run.env_templates, &base);

    let mut command = Command::new(program);
    if let Some(path) = &path {
        command.env("PATH", path);
    }
    command
        .arg("--no-version-check")
        .args(&run.argv)
        .current_dir(&cwd)
        .envs(&run.env)
        .envs(&templates);
    Ok(command)
}

fn run_command<R: Read + Send + 'static>(
    workspace: &Path,
    run: &RunSpec,
    program: &Path,
    mut stdin: R,
) -> Result<i32> {
    let mut command = child_command(workspace, run, program)?;
    purge_stale_runs(workspace);
    let out_dir = run_output_dir(workspace, &run.id);
    fs::create_dir_all(&out_dir)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if run.append_output {
        command.arg("--output").arg(&out_dir);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own process group, so a disconnect stops flutter/gradle/xcodebuild too.
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("Failed to start {}", program.display()))?;

    // The client keeps stdin open for the whole run; EOF means it went away.
    let done = Arc::new(AtomicBool::new(false));
    let pid = child.id();
    {
        let done = done.clone();
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut stdin, &mut std::io::sink());
            if !done.load(Ordering::SeqCst) {
                eprintln!("Client disconnected; stopping the remote run.");
                terminate_group(pid);
            }
        });
    }

    let status = child.wait()?;
    done.store(true, Ordering::SeqCst);
    if !status.success() {
        let _ = fs::remove_dir_all(run_dir(workspace, &run.id));
    }
    Ok(match status.code() {
        Some(EXIT_WORKSPACE_CHANGED) => 1,
        Some(code) => code,
        None => 1,
    })
}

#[cfg(unix)]
fn terminate_group(pid: u32) {
    let group = format!("-{pid}");
    let _ = Command::new("kill").args(["-TERM", "--", &group]).status();
    std::thread::sleep(Duration::from_secs(10));
    let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
}

#[cfg(not(unix))]
fn terminate_group(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .status();
}

/// Handles `fetch`: writes the run's output directory as `.tar.gz` to `out`,
/// then removes the run directory.
pub fn fetch<W: Write>(workspace: &str, run_id: &str, out: W) -> Result<()> {
    validate_run_id(run_id)?;
    let workspace = resolve_workspace(workspace);
    let dir = run_output_dir(&workspace, run_id);
    if !dir.is_dir() {
        return Err(anyhow!(
            "No output for run `{run_id}` in {}",
            workspace.display()
        ));
    }
    let mut builder = tar::Builder::new(GzEncoder::new(out, Compression::fast()));
    builder.follow_symlinks(false);
    append_tree(&mut builder, &dir, &dir)?;
    builder.into_inner()?.finish()?.flush()?;
    let _ = fs::remove_dir_all(run_dir(&workspace, run_id));
    Ok(())
}

fn append_tree<W: Write>(builder: &mut tar::Builder<W>, root: &Path, dir: &Path) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = path.strip_prefix(root)?;
        if entry.file_type()?.is_dir() {
            builder.append_dir(name, &path)?;
            append_tree(builder, root, &path)?;
        } else {
            builder.append_path_with_name(&path, name)?;
        }
    }
    Ok(())
}

/// Builds the `.tar.gz` sync archive of `keys` under `root` into a temporary
/// file, returning it with its length.
pub fn build_archive(root: &Path, keys: &[String]) -> Result<(File, u64)> {
    let file = tempfile::tempfile().context("Failed to create a temporary file")?;
    let mut builder = tar::Builder::new(GzEncoder::new(file, Compression::fast()));
    builder.follow_symlinks(false);
    for key in keys {
        builder
            .append_path_with_name(root.join(key), key)
            .with_context(|| format!("Failed to archive {key}"))?;
    }
    let mut file = builder.into_inner()?.finish()?;
    use std::io::Seek;
    let len = file.stream_position()?;
    file.rewind()?;
    Ok((file, len))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use std::io::Cursor;

    /// Stands in for fastforge: records its arguments and environment.
    fn fake_program(dir: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake-fastforge");
        fs::write(
            &path,
            "#!/bin/sh\n{ pwd; echo \"$@\"; echo \"secret=$SECRET template=$TEMPLATE\"; } > \"$RECORD\"\n\
             for last; do :; done; mkdir -p \"$last/1.0\" && echo artifact > \"$last/1.0/app.zip\"\n",
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Runs `exec` like a client would: stdin stays open until it returns
    /// (EOF earlier means the client disconnected).
    fn run_exec(bytes: Vec<u8>, program: &Path) -> Result<i32> {
        let (reader, mut writer) = std::io::pipe().unwrap();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let feeder = std::thread::spawn(move || {
            let _ = writer.write_all(&bytes);
            let _ = done_rx.recv();
        });
        let result = exec(reader, program);
        done_tx.send(()).unwrap();
        feeder.join().unwrap();
        result
    }

    fn request_bytes(request: &ExecRequest, archive: Option<File>) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(request).unwrap();
        bytes.push(b'\n');
        if let Some(mut file) = archive {
            std::io::copy(&mut file, &mut bytes).unwrap();
        }
        bytes
    }

    #[test]
    fn exec_syncs_runs_and_fetches() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("local");
        fs::create_dir_all(local.join("app/lib")).unwrap();
        fs::write(local.join("app/lib/main.dart"), "main").unwrap();
        let workspace = dir.path().join("ws");
        let record = dir.path().join("record.txt");
        let program = fake_program(dir.path());

        let manifest = Manifest::scan(&local, &[]).unwrap();
        let keys: Vec<String> = manifest.entries.keys().cloned().collect();
        let (archive, len) = build_archive(&local, &keys).unwrap();
        let mut env = BTreeMap::new();
        env.insert("SECRET".to_string(), "s3cret".to_string());
        env.insert("RECORD".to_string(), record.to_string_lossy().into_owned());
        let mut env_templates = BTreeMap::new();
        env_templates.insert("TEMPLATE".to_string(), "$SECRET-x".to_string());
        let request = ExecRequest {
            protocol: PROTOCOL_VERSION,
            workspace: workspace.to_string_lossy().into_owned(),
            base_digest: Some(Manifest::default().digest()),
            manifest: manifest.clone(),
            deleted: vec![],
            archive_len: len,
            run: Some(RunSpec {
                id: "run-1".into(),
                cwd: "app".into(),
                argv: vec!["package".into(), "-t".into(), "zip".into()],
                env,
                env_templates,
                append_output: true,
                interactive: false,
            }),
        };
        let code = run_exec(request_bytes(&request, Some(archive)), &program).unwrap();
        assert_eq!(code, 0);
        assert_eq!(
            fs::read_to_string(workspace.join("app/lib/main.dart")).unwrap(),
            "main"
        );

        let recorded = fs::read_to_string(&record).unwrap();
        let mut lines = recorded.lines();
        assert!(lines.next().unwrap().ends_with("/ws/app"));
        let args = lines.next().unwrap();
        assert!(
            args.starts_with("--no-version-check package -t zip --output "),
            "{args}"
        );
        assert_eq!(lines.next().unwrap(), "secret=s3cret template=s3cret-x");

        // The remote manifest now matches, so a stale base digest is refused.
        assert_eq!(super::manifest(&request.workspace).manifest, manifest);
        let stale = ExecRequest {
            archive_len: 0,
            run: None,
            ..request.clone()
        };
        let code = run_exec(request_bytes(&stale, None), &program).unwrap();
        assert_eq!(code, EXIT_WORKSPACE_CHANGED);

        let mut tarball = Vec::new();
        fetch(&request.workspace, "run-1", &mut tarball).unwrap();
        let mut archive = tar::Archive::new(GzDecoder::new(Cursor::new(tarball)));
        let names: Vec<String> = archive
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"1.0/app.zip".to_string()), "{names:?}");
        assert!(!run_dir(&workspace, "run-1").exists());
        assert!(fetch(&request.workspace, "run-1", Vec::new()).is_err());
    }

    #[test]
    fn runs_wait_for_the_run_lock_but_syncs_do_not() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("ws");
        fs::create_dir_all(state_dir(&workspace)).unwrap();
        let program = fake_program(dir.path());
        let record = dir.path().join("record.txt");
        let request = |run: Option<RunSpec>| ExecRequest {
            protocol: PROTOCOL_VERSION,
            workspace: workspace.to_string_lossy().into_owned(),
            base_digest: None,
            manifest: Manifest::default(),
            deleted: vec![],
            archive_len: 0,
            run,
        };
        let spec = RunSpec {
            id: "run-2".into(),
            cwd: String::new(),
            argv: vec!["package".into()],
            env: [("RECORD".to_string(), record.to_string_lossy().into_owned())].into(),
            env_templates: BTreeMap::new(),
            append_output: true,
            interactive: false,
        };

        // Another run holds the run lock.
        let held = lock_workspace(&workspace, RUN_LOCK).unwrap();

        // A plain sync goes through.
        assert_eq!(
            run_exec(request_bytes(&request(None), None), &program).unwrap(),
            0
        );

        // A run waits until the lock is released.
        let bytes = request_bytes(&request(Some(spec)), None);
        let program2 = program.clone();
        let waiting = std::thread::spawn(move || run_exec(bytes, &program2).unwrap());
        std::thread::sleep(Duration::from_millis(500));
        assert!(!waiting.is_finished(), "the run should wait for the lock");
        assert!(!record.exists());
        drop(held);
        assert_eq!(waiting.join().unwrap(), 0);
        assert!(record.exists());
    }

    #[test]
    fn expands_tilde_entries_in_path() {
        let home = Path::new("/home/me");
        let path = std::ffi::OsString::from("~/development/flutter/bin:/usr/bin:~:/opt/~x");
        assert_eq!(
            expand_path_entries(&path, Some(home)),
            std::ffi::OsString::from("/home/me/development/flutter/bin:/usr/bin:/home/me:/opt/~x")
        );
        assert_eq!(expand_path_entries(&path, None), path);
    }

    #[test]
    fn exec_deletes_and_rejects_unsafe_paths() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("ws");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("old.txt"), "x").unwrap();
        let program = fake_program(dir.path());
        let mut request = ExecRequest {
            protocol: PROTOCOL_VERSION,
            workspace: workspace.to_string_lossy().into_owned(),
            base_digest: None,
            manifest: Manifest::default(),
            deleted: vec!["old.txt".into()],
            archive_len: 0,
            run: None,
        };
        assert_eq!(
            run_exec(request_bytes(&request, None), &program).unwrap(),
            0
        );
        assert!(!workspace.join("old.txt").exists());

        request.deleted = vec!["../escape".into()];
        assert!(run_exec(request_bytes(&request, None), &program).is_err());

        request.deleted = vec![];
        request.protocol = PROTOCOL_VERSION + 1;
        assert!(run_exec(request_bytes(&request, None), &program).is_err());
    }
}
