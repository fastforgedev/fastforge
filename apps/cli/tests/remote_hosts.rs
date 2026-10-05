//! End-to-end tests for remote hosts (`fastforge host`, `package --host`).
//!
//! The hosts use the `local` transport: the "remote" agent is the same
//! compiled binary started through `sh` with its workspaces in a temporary
//! directory, so the whole sync → run → fetch pipeline runs without an SSH
//! server.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

struct Env {
    _dir: TempDir,
    project: PathBuf,
    remote: PathBuf,
    hosts_file: PathBuf,
}

impl Env {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("project");
        let remote = dir.path().join("remote");
        fs::create_dir_all(&project).unwrap();
        let hosts_file = dir.path().join("hosts.yaml");
        fs::write(
            &hosts_file,
            format!(
                "hosts:\n  - name: here\n    transport: local\n    workdir: {}\n    fastforge: {}\n    forward_env: [FASTFORGE_TEST_*]\n    env:\n      FASTFORGE_TEST_TEMPLATE: \"$FASTFORGE_TEST_SECRET-expanded\"\n",
                remote.display(),
                env!("CARGO_BIN_EXE_fastforge"),
            ),
        )
        .unwrap();
        Self {
            project: project.canonicalize().unwrap(),
            remote,
            hosts_file,
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_fastforge"))
            .arg("--no-version-check")
            .args(args)
            .current_dir(&self.project)
            .env("FASTFORGE_HOSTS_FILE", &self.hosts_file)
            .env("FASTFORGE_TEST_SECRET", "s3cret")
            .output()
            .expect("spawn fastforge")
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.project.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// The single workspace directory created on the "remote".
    fn workspace(&self) -> PathBuf {
        let mut entries: Vec<_> = fs::read_dir(&self.remote).unwrap().flatten().collect();
        assert_eq!(entries.len(), 1, "expected one remote workspace");
        entries.pop().unwrap().path()
    }
}

fn text(output: &Output) -> String {
    format!(
        "status: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn exec_syncs_incrementally_and_honours_ignores() {
    let env = Env::new();
    env.write("a.txt", "a");
    env.write("b.txt", "b");
    env.write(".gitignore", "ignored.txt\nkey.properties\n");
    env.write(".fastforgeignore", "!key.properties\n");
    env.write("ignored.txt", "x");
    env.write("key.properties", "k");
    env.write("bin/tool.sh", "#!/bin/sh\necho tool ran\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            env.project.join("bin/tool.sh"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }

    let out = env.run(&[
        "host",
        "exec",
        "here",
        "--",
        "./bin/tool.sh",
        "&&",
        "ls",
        "-A",
    ]);
    assert!(out.status.success(), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("tool ran"), "{}", text(&out));
    assert!(stdout.contains("a.txt") && stdout.contains("b.txt"));
    assert!(stdout.contains("key.properties"));
    assert!(!stdout.contains("ignored.txt"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("6 changed, 0 deleted"), "{}", text(&out));

    // Second sync: nothing to send.
    let out = env.run(&["host", "exec", "here", "--", "true"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("0 changed, 0 deleted"),
        "{}",
        text(&out)
    );

    // A file edited on the remote side is sent again; deletions propagate.
    let workspace = env.workspace();
    fs::write(workspace.join("a.txt"), "edited remotely, longer").unwrap();
    fs::remove_file(env.project.join("b.txt")).unwrap();
    env.write("c.txt", "c");
    let out = env.run(&["host", "exec", "here", "--", "ls", "-A"]);
    assert!(out.status.success(), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("b.txt") && stdout.contains("c.txt"),
        "{}",
        text(&out)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("2 changed, 1 deleted"),
        "{}",
        text(&out)
    );
    assert_eq!(fs::read_to_string(workspace.join("a.txt")).unwrap(), "a");
}

#[test]
fn exec_propagates_the_exit_code() {
    let env = Env::new();
    env.write("a.txt", "a");
    let out = env.run(&["host", "exec", "here", "--", "exit", "3"]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
}

#[test]
fn doctor_detects_and_saves_platforms() {
    let env = Env::new();
    let out = env.run(&["host", "doctor", "here"]);
    assert!(out.status.success(), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("connection"), "{}", text(&out));
    assert!(stdout.contains("remote protocol 1"), "{}", text(&out));
    let saved = fs::read_to_string(&env.hosts_file).unwrap();
    assert!(saved.contains("platforms:"), "{saved}");
    assert!(saved.contains("- android"), "{saved}");

    let out = env.run(&["host", "list"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("here"),
        "{}",
        text(&out)
    );
}

#[test]
fn host_add_and_remove() {
    let env = Env::new();
    let out = env.run(&[
        "host",
        "add",
        "mac",
        "builder@10.0.0.9",
        "--platforms",
        "ios",
    ]);
    assert!(out.status.success(), "{}", text(&out));
    let out = env.run(&["host", "add", "mac", "other@10.0.0.9"]);
    assert!(!out.status.success(), "duplicate host should fail");
    let saved = fs::read_to_string(&env.hosts_file).unwrap();
    assert!(
        saved.contains("user: builder") && saved.contains("- ios"),
        "{saved}"
    );
    let out = env.run(&["host", "remove", "mac"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        !fs::read_to_string(&env.hosts_file)
            .unwrap()
            .contains("builder")
    );
}

#[test]
fn package_reports_remote_failures_and_unknown_hosts() {
    let env = Env::new();
    env.write("README.md", "not a flutter project");
    let out = env.run(&["package", "-p", "linux", "-t", "deb", "--host", "here"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("No pubspec.yaml found"), "{}", text(&out));
    assert!(
        stderr.contains("remote run on `here` failed"),
        "{}",
        text(&out)
    );
    // The failed run's directory is cleaned up.
    let runs = env.workspace().join(".fastforge/remote/runs");
    assert_eq!(fs::read_dir(runs).map(|d| d.count()).unwrap_or(0), 0);

    let out = env.run(&["package", "-t", "deb", "--host", "nope"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Unknown host `nope`"),
        "{}",
        text(&out)
    );
    let out = env.run(&["package", "-t", "zip", "--host", "auto"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--platform"),
        "{}",
        text(&out)
    );
}

/// Real Flutter build on the "remote": `package -p web -t zip --host here`
/// must leave the zip in the local `dist/` and nothing behind remotely. The
/// app comes from `flutter create`, so it builds with whatever SDK is installed.
#[test]
fn flutter_web_zip_is_packaged_remotely_and_fetched() {
    let env = Env::new();
    let created = Command::new("flutter")
        .args([
            "create",
            "--platforms",
            "web",
            "--offline",
            "--project-name",
            "remote_app",
            ".",
        ])
        .current_dir(&env.project)
        .output();
    match created {
        Ok(output) if output.status.success() => {}
        _ => {
            eprintln!("skipping: `flutter create` is unavailable");
            return;
        }
    }

    let out = env.run(&[
        "package",
        "-p",
        "web",
        "-t",
        "zip",
        "--host",
        "here",
        "--skip-clean",
    ]);
    assert!(out.status.success(), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Fetched 1 artifact(s) from here"),
        "{}",
        text(&out)
    );

    let zips: Vec<PathBuf> = walk(&env.project.join("dist"))
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "zip"))
        .collect();
    assert_eq!(zips.len(), 1, "{}", text(&out));

    let workspace = env.workspace();
    assert!(workspace.join("lib/main.dart").is_file());
    assert!(
        !workspace.join("dist").exists(),
        "artifacts go to the run dir, not dist/"
    );
    let runs = workspace.join(".fastforge/remote/runs");
    assert_eq!(fs::read_dir(runs).map(|d| d.count()).unwrap_or(0), 0);
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

/// A fake `flutter` on `PATH`: lists two devices, echoes its arguments and a
/// VM service URL, then answers `r` (printing a project file, to observe
/// syncs) and `q` on stdin.
fn fake_flutter(env: &Env) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let bin = env.remote.parent().unwrap().join("fake-bin");
    fs::create_dir_all(&bin).unwrap();
    let flutter = bin.join("flutter");
    fs::write(
        &flutter,
        r#"#!/bin/sh
if [ "$1" = devices ]; then
  echo 'Checking devices...'
  echo '[{"name":"macOS","id":"macos","isSupported":true,"targetPlatform":"darwin"},{"name":"Web Server","id":"web-server","isSupported":true,"targetPlatform":"web-javascript"}]'
  exit 0
fi
echo "flutter args: $*"
echo "A Dart VM Service is available at: http://127.0.0.1:54321/abc=/"
while IFS= read -r line; do
  case "$line" in
    r) echo "reload sees: $(cat lib/version.txt)" ;;
    q) echo "bye"; exit 0 ;;
  esac
done
"#,
    )
    .unwrap();
    fs::set_permissions(&flutter, fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn run_command(env: &Env, bin: &Path, args: &[&str]) -> Command {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_fastforge"));
    command
        .arg("--no-version-check")
        .args(args)
        .current_dir(&env.project)
        .env("FASTFORGE_HOSTS_FILE", &env.hosts_file)
        .env("PATH", path)
        .env_remove("FLUTTER_ROOT");
    command
}

#[test]
fn run_selects_a_device_and_passes_flutter_arguments() {
    let env = Env::new();
    env.write("pubspec.yaml", "name: app\n");
    let bin = fake_flutter(&env);
    let out = run_command(
        &env,
        &bin,
        &["run", "-p", "macos", "--release", "--", "--verbose"],
    )
    .stdin(std::process::Stdio::null())
    .output()
    .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains("flutter args: run -d macos --release --verbose"),
        "{}",
        text(&out)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("Using device macOS (macos)"));

    let out = run_command(&env, &bin, &["run", "-p", "ios"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("No `ios` device found"),
        "{}",
        text(&out)
    );

    fs::remove_file(env.project.join("pubspec.yaml")).unwrap();
    let out = run_command(&env, &bin, &["run"]).output().unwrap();
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Flutter projects only"),
        "{}",
        text(&out)
    );
}

#[test]
fn remote_run_syncs_before_hot_reload() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;

    let env = Env::new();
    env.write("pubspec.yaml", "name: app\n");
    env.write("lib/version.txt", "v1");
    let bin = fake_flutter(&env);
    let mut child = run_command(&env, &bin, &["run", "-p", "web", "--host", "here"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (tx, rx) = mpsc::channel();
    let stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    let wait_for = |needle: &str| -> Vec<String> {
        let mut seen = Vec::new();
        loop {
            match rx.recv_timeout(Duration::from_secs(60)) {
                Ok(line) => {
                    let done = line.contains(needle);
                    seen.push(line);
                    if done {
                        return seen;
                    }
                }
                Err(_) => panic!("timed out waiting for `{needle}`; got {seen:?}"),
            }
        }
    };

    let started = wait_for("VM Service");
    assert!(
        started
            .iter()
            .any(|l| l.contains("flutter args: run -d web-server")),
        "{started:?}"
    );
    env.write("lib/version.txt", "v2");
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"r\n").unwrap();
    stdin.flush().unwrap();
    wait_for("reload sees: v2");
    stdin.write_all(b"q\n").unwrap();
    wait_for("bye");
    drop(stdin);

    let status = child.wait().unwrap();
    let mut stderr = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut stderr).unwrap();
    assert!(status.success(), "{stderr}");
    assert!(stderr.contains("synced 1 changed, 0 deleted"), "{stderr}");
    // The run directory (with its stored spec) is gone.
    let runs = env.workspace().join(".fastforge/remote/runs");
    assert_eq!(fs::read_dir(runs).map(|d| d.count()).unwrap_or(0), 0);
}
