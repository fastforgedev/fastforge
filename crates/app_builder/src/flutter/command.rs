use crate::flutter::FlutterVersion;
use fastforge_core::{BuildError, path_expansion};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct FlutterCommand<'a> {
    environment: Option<&'a HashMap<String, String>>,
}

impl<'a> FlutterCommand<'a> {
    pub fn new(environment: Option<&'a HashMap<String, String>>) -> Self {
        Self { environment }
    }

    /// Runs `flutter clean`, streaming its output. Like the Dart CLI, a
    /// failing clean does not abort the build.
    pub fn clean(&self) -> Result<(), BuildError> {
        let mut cmd = self.base_command()?;
        cmd.arg("clean");
        echo_command("flutter clean");
        let status = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| BuildError::Io(format!("Failed to execute flutter clean: {}", e)))?;
        if !status.success() {
            eprintln!(
                "\x1b[33mWarning: flutter clean exited with code {}\x1b[0m",
                status.code().unwrap_or(-1)
            );
        }
        Ok(())
    }

    /// Runs `flutter build <subcommand> <arguments>`, streaming its output.
    /// Returns the exit code and the captured stderr (Dart's `BuildError`
    /// carries the stderr of a failed build).
    pub fn build(
        &self,
        subcommand: &str,
        arguments: &[String],
    ) -> Result<(i32, String), BuildError> {
        let mut cmd = self.base_command()?;
        cmd.arg("build").arg(subcommand).args(arguments);
        cmd.stdout(Stdio::inherit()).stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| BuildError::Io(format!("Failed to execute flutter build: {}", e)))?;
        let mut captured = String::new();
        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr);
            let mut line = Vec::new();
            let mut sink = std::io::stderr();
            while reader.read_until(b'\n', &mut line).unwrap_or(0) > 0 {
                let _ = sink.write_all(&line);
                let _ = sink.flush();
                captured.push_str(&String::from_utf8_lossy(&line));
                line.clear();
            }
        }
        let status = child
            .wait()
            .map_err(|e| BuildError::Io(format!("Failed to execute flutter build: {}", e)))?;
        Ok((status.code().unwrap_or(-1), captured))
    }

    pub fn build_with_echo(
        &self,
        subcommand: &str,
        arguments: &[String],
    ) -> Result<(i32, String), BuildError> {
        echo_command(&format!(
            "flutter build {} {}",
            subcommand,
            arguments.join(" ")
        ));
        self.build(subcommand, arguments)
    }

    pub fn version(&self) -> Result<FlutterVersion, BuildError> {
        let mut cmd = self.base_command()?;
        cmd.arg("--version").arg("--machine");
        let output = cmd
            .output()
            .map_err(|e| BuildError::Io(format!("Failed to read flutter version: {}", e)))?;
        if !output.status.success() {
            return Err(BuildError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        let parsed: Value = serde_json::from_slice(&output.stdout).map_err(|e| {
            BuildError::Parse(format!(
                "Failed to parse flutter --version --machine JSON: {}",
                e
            ))
        })?;
        let flutter_version = parsed
            .get("flutterVersion")
            .or_else(|| parsed.get("frameworkVersion"))
            .and_then(Value::as_str)
            .map(ToString::to_string);
        Ok(FlutterVersion { flutter_version })
    }

    /// A `flutter` command (resolved through `FLUTTER_ROOT` when set) with the
    /// environment applied, for callers that run other subcommands.
    pub fn base_command(&self) -> Result<Command, BuildError> {
        let executable = self.resolve_executable()?;
        let mut cmd = Command::new(executable);
        if let Some(env) = self.environment {
            cmd.envs(env);
        }
        Ok(cmd)
    }

    fn resolve_executable(&self) -> Result<String, BuildError> {
        // On Windows `flutter` is a batch script; Dart ran it through the
        // shell (`runInShell: true`), Rust has to name the `.bat` explicitly.
        let file_name = if cfg!(windows) {
            "flutter.bat"
        } else {
            "flutter"
        };
        if let Some(env) = self.environment
            && let Some(root) = env.get("FLUTTER_ROOT")
            && !root.is_empty()
        {
            let root = path_expansion(root, env);
            if !PathBuf::from(&root).is_dir() {
                return Err(BuildError::Io(format!(
                    "FLUTTER_ROOT environment variable is set to a path that does not exist: {}",
                    root
                )));
            }
            let path = PathBuf::from(root).join("bin").join(file_name);
            return Ok(path.to_string_lossy().to_string());
        }
        Ok(file_name.to_string())
    }
}

/// Prints the command line before running it (Dart's `DefaultShellExecutor`
/// logs `$ <command>` in bright black).
fn echo_command(command_line: &str) {
    eprintln!("\x1b[90m$ {}\x1b[0m", command_line);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn uses_flutter_root_when_set() {
        let dir = tempdir().expect("tempdir");
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("mkdir");
        let flutter = bin_dir.join("flutter");
        fs::write(&flutter, "#!/bin/sh\nexit 0\n").expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = fs::metadata(&flutter).expect("meta").permissions();
            perm.set_mode(0o755);
            fs::set_permissions(&flutter, perm).expect("chmod");
        }

        let mut env = HashMap::new();
        env.insert(
            "FLUTTER_ROOT".to_string(),
            dir.path().to_string_lossy().to_string(),
        );
        let command = FlutterCommand::new(Some(&env));
        let resolved = command.resolve_executable().expect("resolve executable");
        assert!(resolved.contains("bin") && resolved.contains("flutter"));
    }

    #[test]
    fn expands_flutter_root() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("sdk")).expect("mkdir");
        let mut env = HashMap::new();
        env.insert("SDKS".to_string(), dir.path().to_string_lossy().to_string());
        env.insert("FLUTTER_ROOT".to_string(), "${SDKS}/sdk".to_string());
        let command = FlutterCommand::new(Some(&env));
        let resolved = command.resolve_executable().expect("resolve executable");
        assert!(resolved.starts_with(&*dir.path().to_string_lossy()));
    }

    #[test]
    fn fails_when_flutter_root_does_not_exist() {
        let mut env = HashMap::new();
        env.insert(
            "FLUTTER_ROOT".to_string(),
            "/path/that/does/not/exist".to_string(),
        );
        let command = FlutterCommand::new(Some(&env));
        let err = command.resolve_executable().expect_err("must fail");
        assert!(
            err.to_string()
                .contains("FLUTTER_ROOT environment variable is set to a path that does not exist")
        );
    }
}
