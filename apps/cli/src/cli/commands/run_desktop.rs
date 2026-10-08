//! `fastforge run -d windows` from a session without a desktop.
//!
//! Programs started over SSH on Windows live in session 0, which has no
//! visible desktop, so a Flutter window can't appear. Instead the app is
//! built here, started in the logged-in user's desktop session through an
//! interactive scheduled task, and `flutter attach` connects to it for hot
//! reload. The app is stopped when this command ends.

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use fastforge_app_builder::FlutterCommand;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::run::RunArgs;
use crate::utils::bright_black;

/// Set to `0` to never use the desktop session, or `1` to always use it.
const DESKTOP_LAUNCH_ENV: &str = "FASTFORGE_DESKTOP_LAUNCH";

/// Kinds of [`launch`]: the app, and the Host of its remote window.
const LAUNCH_KINDS: &[&str] = &["run", "window"];

/// How long the logged-in session gets to start the app.
const START_TIMEOUT: Duration = Duration::from_secs(30);

/// How long the app gets to print its VM service URL.
const VM_SERVICE_TIMEOUT: Duration = Duration::from_secs(60);

/// Whether a Windows app run from this process would get no visible window.
/// Interactive sessions (console, RDP) set `SESSIONNAME`; session 0, where
/// the OpenSSH server starts commands, does not.
pub fn needs_desktop_session() -> bool {
    match std::env::var(DESKTOP_LAUNCH_ENV).as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        _ => cfg!(windows) && std::env::var_os("SESSIONNAME").is_none(),
    }
}

/// The display variable that puts a Linux app on the logged-in user's
/// desktop when this session has none (e.g. over SSH): the user's Wayland
/// socket in `runtime_dir`, else the X server `:0`.
pub fn linux_display_in(runtime_dir: Option<&Path>, x11_dir: &Path) -> Option<(String, String)> {
    if let Some(dir) = runtime_dir
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        let mut sockets: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with("wayland-") && !n.ends_with(".lock"))
            .collect();
        sockets.sort();
        if let Some(socket) = sockets.into_iter().next() {
            return Some(("WAYLAND_DISPLAY".to_string(), socket));
        }
    }
    x11_dir
        .join("X0")
        .exists()
        .then(|| ("DISPLAY".to_string(), ":0".to_string()))
}

/// [`linux_display_in`] for this process, when running a Linux desktop app
/// without `DISPLAY`/`WAYLAND_DISPLAY`.
pub fn linux_display_env(device: Option<&str>) -> Option<(String, String)> {
    if !cfg!(target_os = "linux")
        || device != Some("linux")
        || std::env::var_os("DISPLAY").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
    {
        return None;
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    linux_display_in(runtime.as_deref(), Path::new("/tmp/.X11-unix"))
}

/// `BINARY_NAME` from `windows/CMakeLists.txt`.
pub fn binary_name(cmake: &str) -> Option<String> {
    cmake.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("set(BINARY_NAME")?;
        let value = rest.trim().trim_end_matches(')').trim();
        let value = value.trim_matches('"');
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Build mode directory name and `flutter build` flag.
fn build_mode(args: &RunArgs) -> (&'static str, &'static str) {
    if args.release {
        ("Release", "--release")
    } else if args.profile {
        ("Profile", "--profile")
    } else {
        ("Debug", "--debug")
    }
}

/// The built executable: `build/windows/<arch>/runner/<Mode>/<name>.exe`.
fn find_executable(root: &Path, name: &str, mode: &str) -> Result<PathBuf> {
    for arch in ["x64", "arm64"] {
        let exe = root
            .join("build")
            .join("windows")
            .join(arch)
            .join("runner")
            .join(mode)
            .join(format!("{name}.exe"));
        if exe.is_file() {
            return Ok(exe);
        }
    }
    bail!("Built app `{name}.exe` not found under build/windows")
}

/// Quotes one argument for a Windows command line, as the C runtime parses
/// it: plain when it has no blanks or quotes, else in double quotes with
/// inner quotes and the backslashes before them escaped.
fn windows_arg(value: &str) -> String {
    if !value.is_empty() && !value.contains([' ', '\t', '"']) {
        return value.to_string();
    }
    let mut quoted = String::from('"');
    let mut backslashes = 0;
    for c in value.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push(c);
                backslashes = 0;
            }
        }
    }
    // Backslashes before the closing quote are doubled so they stay literal.
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

/// Quotes a string for PowerShell (single quotes, doubled inside).
fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Files of one desktop launch.
struct LaunchFiles {
    dir: PathBuf,
    script: PathBuf,
    log: PathBuf,
    err: PathBuf,
    pid: PathBuf,
}

impl LaunchFiles {
    fn new(dir: PathBuf) -> Self {
        Self {
            script: dir.join("launch.ps1"),
            log: dir.join("app.log"),
            err: dir.join("app.err.log"),
            pid: dir.join("app.pid"),
            dir,
        }
    }
}

/// The script the scheduled task runs in the desktop session: starts `exe`
/// with `args` and `env` (the app with a VM service on a free port when
/// `debug`), records its pid, and stops it once `owner` (this process) is
/// gone.
fn launch_script(
    exe: &Path,
    args: &[String],
    env: &[(&str, &str)],
    cwd: &Path,
    files: &LaunchFiles,
    owner: u32,
    debug: bool,
) -> String {
    let mut script = String::from("$ErrorActionPreference = 'Stop'\n");
    for (key, value) in env {
        script.push_str(&format!("$env:{key} = {}\n", ps_quote(value)));
    }
    if debug {
        script.push_str(
            "$env:FLUTTER_ENGINE_SWITCHES = '2'\n\
             $env:FLUTTER_ENGINE_SWITCH_1 = 'vm-service-port=0'\n\
             $env:FLUTTER_ENGINE_SWITCH_2 = 'disable-service-auth-codes'\n",
        );
    }
    let arguments = if args.is_empty() {
        String::new()
    } else {
        let line: Vec<String> = args.iter().map(|a| windows_arg(a)).collect();
        format!(" -ArgumentList {}", ps_quote(&line.join(" ")))
    };
    script.push_str(&format!(
        "$app = Start-Process -FilePath {exe}{arguments} -WorkingDirectory {cwd} -RedirectStandardOutput {log} -RedirectStandardError {err} -PassThru\n\
         Set-Content -Path {pid} -Value $app.Id\n\
         while (-not $app.HasExited) {{\n\
         \x20 if (-not (Get-Process -Id {owner} -ErrorAction SilentlyContinue)) {{ Stop-Process -Id $app.Id -Force; break }}\n\
         \x20 Start-Sleep -Milliseconds 500\n\
         }}\n",
        exe = ps_quote(&exe.to_string_lossy()),
        cwd = ps_quote(&cwd.to_string_lossy()),
        log = ps_quote(&files.log.to_string_lossy()),
        err = ps_quote(&files.err.to_string_lossy()),
        pid = ps_quote(&files.pid.to_string_lossy()),
    ));
    script
}

/// Registers an interactive task (no trigger) running `script` in the
/// user's desktop session, and starts it.
fn start_task_script(task: &str, script: &Path) -> String {
    let arguments = format!(
        "--headless powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File \"{}\"",
        script.display()
    );
    format!(
        "$ErrorActionPreference = 'Stop'\n\
         $action = New-ScheduledTaskAction -Execute 'conhost.exe' -Argument {arguments}\n\
         $user = [Security.Principal.WindowsIdentity]::GetCurrent().Name\n\
         $principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited\n\
         $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero)\n\
         Register-ScheduledTask -TaskName {task} -Action $action -Principal $principal -Settings $settings -Force | Out-Null\n\
         Start-ScheduledTask -TaskName {task}\n",
        arguments = ps_quote(&arguments),
        task = ps_quote(task),
    )
}

/// Runs `script` with Windows PowerShell, with plain-text UTF-8 output (it
/// otherwise reports errors as CLIXML in the console code page).
fn powershell(script: &str) -> Result<std::process::Output> {
    let script = format!(
        "$ProgressPreference = 'SilentlyContinue'\n\
         [Console]::OutputEncoding = [Text.Encoding]::UTF8\n\
         try {{\n{script}\n}} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}\n"
    );
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-OutputFormat",
            "Text",
            "-EncodedCommand",
            &encoded,
        ])
        .stdin(Stdio::null())
        .output()
        .context("Failed to run powershell.exe")
}

fn remove_task(task: &str) {
    let _ = powershell(&format!(
        "Unregister-ScheduledTask -TaskName {} -Confirm:$false -ErrorAction SilentlyContinue",
        ps_quote(task)
    ));
}

fn kill(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn is_running(pid: u32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
}

/// The VM service URL the app prints at startup.
pub fn vm_service_url(log: &str) -> Option<String> {
    let start = log.find("The Dart VM service is listening on ")? + 36;
    let url: String = log[start..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    url.starts_with("http").then_some(url)
}

/// A program started in the desktop session; stopped (if still running)
/// and its files removed when dropped.
pub struct DesktopApp {
    pub pid: u32,
    files: LaunchFiles,
}

impl DesktopApp {
    pub fn is_running(&self) -> bool {
        is_running(self.pid)
    }

    /// What the program has written to stdout and stderr so far.
    pub fn output(&self) -> (String, String) {
        let read = |path: &Path| std::fs::read_to_string(path).unwrap_or_default();
        (read(&self.files.log), read(&self.files.err))
    }
}

impl Drop for DesktopApp {
    fn drop(&mut self) {
        if is_running(self.pid) {
            kill(self.pid);
        }
        remove_dir_eventually(&self.files.dir);
    }
}

/// Removes `dir`, retrying while the launcher script still holds its files
/// open (Windows refuses to delete open files).
fn remove_dir_eventually(dir: &Path) {
    for _ in 0..20 {
        if std::fs::remove_dir_all(dir).is_ok() || !dir.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Removes launch directories (`fastforge-<kind>-<pid>`) left by runs whose
/// process is gone (for example after a dropped connection).
fn purge_stale_launches(temp: &Path) {
    let Ok(entries) = std::fs::read_dir(temp) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix("fastforge-"))
            .and_then(|n| n.split_once('-'))
            .filter(|(kind, _)| LAUNCH_KINDS.contains(kind))
            .and_then(|(_, p)| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != std::process::id() && !is_running(pid) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Starts `exe` in the logged-in user's desktop session with `args` and
/// the variables in `env`; `kind` names the launch (`run` for the app) and
/// keeps concurrent launches apart.
pub fn launch(
    kind: &str,
    exe: &Path,
    args: &[String],
    env: &[(&str, &str)],
    cwd: &Path,
    debug: bool,
) -> Result<DesktopApp> {
    debug_assert!(LAUNCH_KINDS.contains(&kind));
    let owner = std::process::id();
    let task = format!("fastforge-{kind}-{owner}");
    purge_stale_launches(&std::env::temp_dir());
    let files = LaunchFiles::new(std::env::temp_dir().join(&task));
    std::fs::create_dir_all(&files.dir)?;
    std::fs::write(
        &files.script,
        launch_script(exe, args, env, cwd, &files, owner, debug),
    )?;
    let name = exe.file_name().unwrap_or(exe.as_os_str()).to_string_lossy();

    let output = powershell(&start_task_script(&task, &files.script))?;
    if !output.status.success() {
        remove_task(&task);
        let _ = std::fs::remove_dir_all(&files.dir);
        bail!(
            "Could not start {name} in the desktop session: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let started = Instant::now();
    let pid = loop {
        if let Some(pid) = std::fs::read_to_string(&files.pid)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            break pid;
        }
        if started.elapsed() > START_TIMEOUT {
            remove_task(&task);
            let _ = std::fs::remove_dir_all(&files.dir);
            bail!(
                "{name} did not start in a desktop session. Log in to this Windows machine's desktop \
                 (a locked screen is fine){}.",
                if kind == "run" {
                    ", or use `-p web` instead"
                } else {
                    ""
                }
            );
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    // The task has done its job; the script keeps running without it.
    remove_task(&task);
    eprintln!(
        "Started {} in the desktop session (pid {pid}).",
        exe.display()
    );
    Ok(DesktopApp { pid, files })
}

fn wait_for_vm_service(app: &DesktopApp) -> Result<String> {
    let started = Instant::now();
    loop {
        let log = std::fs::read_to_string(&app.files.log).unwrap_or_default();
        if let Some(url) = vm_service_url(&log) {
            return Ok(url);
        }
        if !is_running(app.pid) {
            let err = std::fs::read_to_string(&app.files.err).unwrap_or_default();
            bail!("The app exited during startup.\n{log}{err}");
        }
        if started.elapsed() > VM_SERVICE_TIMEOUT {
            bail!("The app did not report its VM service URL in time.\n{log}");
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// Prints the app's output as it grows until it exits or `stop` is set.
fn follow_output(app: &DesktopApp, stop: &AtomicBool) {
    let mut shown = [0usize; 2];
    loop {
        for (i, path) in [&app.files.log, &app.files.err].into_iter().enumerate() {
            if let Ok(bytes) = std::fs::read(path)
                && bytes.len() > shown[i]
            {
                print!("{}", String::from_utf8_lossy(&bytes[shown[i]..]));
                shown[i] = bytes.len();
            }
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
        if stop.load(Ordering::SeqCst) || !is_running(app.pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// Builds, launches in the desktop session and attaches (debug) or follows
/// the output (profile/release) until the app or this command ends.
pub async fn run(args: &RunArgs, flutter: &FlutterCommand<'_>) -> Result<()> {
    let root = std::env::current_dir()?;
    let cmake = std::fs::read_to_string(root.join("windows/CMakeLists.txt")).context(
        "windows/CMakeLists.txt not found; is this a Flutter project with Windows support?",
    )?;
    let name = binary_name(&cmake)
        .ok_or_else(|| anyhow!("BINARY_NAME not set in windows/CMakeLists.txt"))?;
    let (mode, mode_flag) = build_mode(args);
    let debug = mode == "Debug";

    eprintln!("This session has no desktop; the app will open on the logged-in user's desktop.");
    let mut build_args = vec![
        "build".to_string(),
        "windows".to_string(),
        mode_flag.to_string(),
    ];
    build_args.extend(args.compile_args(true));
    eprintln!(
        "{}",
        bright_black(&format!("$ flutter {}", build_args.join(" ")))
    );
    let status = flutter
        .base_command()
        .map_err(|e| anyhow!("{e}"))?
        .args(&build_args)
        .status()
        .context("Failed to start `flutter build`")?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    let exe = find_executable(&root, &name, mode)?;
    let app = tokio::task::spawn_blocking({
        let root = root.clone();
        move || launch("run", &exe, &[], &[], &root, debug)
    })
    .await??;

    if !debug {
        // No VM service outside debug builds: show the output until the app
        // closes or Ctrl-C.
        let stop = Arc::new(AtomicBool::new(false));
        let watcher = {
            let stop = stop.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    stop.store(true, Ordering::SeqCst);
                }
            })
        };
        tokio::task::spawn_blocking(move || {
            follow_output(&app, &stop);
            drop(app);
        })
        .await?;
        watcher.abort();
        return Ok(());
    }

    let app = Arc::new(app);
    let url = {
        let app = app.clone();
        tokio::task::spawn_blocking(move || wait_for_vm_service(&app)).await??
    };
    let mut attach_args = vec![
        "attach".to_string(),
        "-d".to_string(),
        "windows".to_string(),
        "--debug-url".to_string(),
        url,
    ];
    attach_args.extend(args.compile_args(false));
    attach_args.extend(args.flutter_args.iter().cloned());
    eprintln!(
        "{}",
        bright_black(&format!("$ flutter {}", attach_args.join(" ")))
    );
    let mut child = flutter
        .base_command()
        .map_err(|e| anyhow!("{e}"))?
        .args(&attach_args)
        .spawn()
        .context("Failed to start `flutter attach`")?;
    let guard = tokio::spawn(async { while tokio::signal::ctrl_c().await.is_ok() {} });
    let status = tokio::task::spawn_blocking(move || child.wait()).await??;
    guard.abort();
    drop(app);
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_wayland_socket_then_x11() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = dir.path().join("run");
        let x11 = dir.path().join("x11");
        std::fs::create_dir_all(&runtime).unwrap();
        std::fs::create_dir_all(&x11).unwrap();
        assert_eq!(linux_display_in(Some(&runtime), &x11), None);

        std::fs::write(x11.join("X0"), "").unwrap();
        assert_eq!(
            linux_display_in(Some(&runtime), &x11),
            Some(("DISPLAY".into(), ":0".into()))
        );

        std::fs::write(runtime.join("wayland-0.lock"), "").unwrap();
        std::fs::write(runtime.join("wayland-0"), "").unwrap();
        assert_eq!(
            linux_display_in(Some(&runtime), &x11),
            Some(("WAYLAND_DISPLAY".into(), "wayland-0".into()))
        );
        assert_eq!(
            linux_display_in(None, &x11),
            Some(("DISPLAY".into(), ":0".into()))
        );
    }

    #[test]
    fn reads_the_binary_name() {
        let cmake = "cmake_minimum_required(VERSION 3.14)\nproject(winapp LANGUAGES CXX)\n\nset(BINARY_NAME \"winapp\")\n";
        assert_eq!(binary_name(cmake).as_deref(), Some("winapp"));
        assert_eq!(
            binary_name("set(BINARY_NAME my_app)").as_deref(),
            Some("my_app")
        );
        assert_eq!(binary_name("project(x)"), None);
    }

    #[test]
    fn quotes_windows_arguments() {
        assert_eq!(windows_arg("winapp"), "winapp");
        assert_eq!(windows_arg(""), r#""""#);
        assert_eq!(windows_arg("my app"), r#""my app""#);
        assert_eq!(windows_arg(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(windows_arg(r"C:\my dir\"), r#""C:\my dir\\""#);
        assert_eq!(windows_arg(r#"a\"b"#), r#""a\\\"b""#);
    }

    #[test]
    fn finds_the_vm_service_url() {
        let log = "[IMPORTANT:flutter/...] Using the Impeller rendering backend.\nThe Dart VM service is listening on http://127.0.0.1:50505/\n";
        assert_eq!(
            vm_service_url(log).as_deref(),
            Some("http://127.0.0.1:50505/")
        );
        assert_eq!(vm_service_url("starting..."), None);
    }

    #[test]
    fn launch_script_quotes_paths_and_watches_the_owner() {
        let files = LaunchFiles::new(PathBuf::from(r"C:\Temp\fastforge-run-7"));
        let script = launch_script(
            Path::new(r"C:\Users\O'Neil\app\build\winapp.exe"),
            &[],
            &[],
            Path::new(r"C:\Users\O'Neil\app"),
            &files,
            42,
            true,
        );
        assert!(script.contains(r"-FilePath 'C:\Users\O''Neil\app\build\winapp.exe'"));
        assert!(script.contains("vm-service-port=0"));
        assert!(script.contains("Get-Process -Id 42"));
        assert!(!script.contains("-ArgumentList"));
        let script = launch_script(
            Path::new("dazzdesk.exe"),
            &["--filter".into(), "O'Neil app".into()],
            &[("RUST_LOG", "info")],
            Path::new("."),
            &files,
            1,
            false,
        );
        assert!(!script.contains("ENGINE_SWITCH"));
        assert!(script.contains(r#"-ArgumentList '--filter "O''Neil app"'"#));
        assert!(script.contains("$env:RUST_LOG = 'info'"));

        let task = start_task_script("fastforge-run-7", &files.script);
        assert!(task.contains("-LogonType Interactive"));
        assert!(task.contains("--headless powershell.exe"));
        assert!(task.contains("Start-ScheduledTask -TaskName 'fastforge-run-7'"));
    }
}
