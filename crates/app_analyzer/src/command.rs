use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Upper bound for every helper process spawned while analyzing a macOS
/// artifact. Tools such as `spctl` and `stapler` may reach the network, so a
/// hard cap keeps `fastforge analyze` from blocking forever on them.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct CommandOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CommandOutput {
    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    /// `codesign` and `spctl` report to stderr, `stapler` to stdout; callers
    /// that only care about the human-readable report scan both.
    pub fn combined_text(&self) -> String {
        let mut text = self.stdout_text();
        let stderr = self.stderr_text();
        if !stderr.is_empty() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&stderr);
        }
        text
    }
}

/// Runs `program` with [`DEFAULT_TIMEOUT`], returning `None` when the tool is
/// missing, could not be spawned, or exceeded its time budget.
pub fn run(program: &str, args: &[&str]) -> Option<CommandOutput> {
    run_with_timeout(program, args, DEFAULT_TIMEOUT)
}

pub fn run_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<CommandOutput> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| log::debug!("failed to spawn `{}`: {}", program, e))
        .ok()?;

    // Drain both pipes from separate threads: waiting on the child first would
    // deadlock as soon as either pipe buffer fills up.
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || read_all(&mut stdout_pipe));
    let stderr_reader = std::thread::spawn(move || read_all(&mut stderr_pipe));

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    log::warn!("`{}` timed out after {:?}, killing it", program, timeout);
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                log::debug!("failed to wait for `{}`: {}", program, e);
                return None;
            }
        }
    };

    Some(CommandOutput {
        success: status.success(),
        stdout: stdout_reader.join().unwrap_or_default(),
        stderr: stderr_reader.join().unwrap_or_default(),
    })
}

fn read_all<R: Read>(pipe: &mut Option<R>) -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(reader) = pipe.as_mut() {
        let _ = reader.read_to_end(&mut buf);
    }
    buf
}
