use crate::flutter::FlutterVersion;
use fastforge_core::{BuildError, path_expansion};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
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
            let bin_dir = PathBuf::from(root).join("bin");
            if cfg!(windows) {
                let extensions = windows_flutter_extensions(
                    env_var_case_insensitive(self.environment, "PATHEXT").as_deref(),
                );
                let resolved = find_windows_flutter_in_dir(&bin_dir, &extensions)
                    .unwrap_or_else(|| bin_dir.join("flutter.bat"));
                return Ok(resolved.to_string_lossy().to_string());
            }
            return Ok(bin_dir.join("flutter").to_string_lossy().to_string());
        }

        if cfg!(windows) {
            return Ok(resolve_windows_flutter(self.environment));
        }
        Ok("flutter".to_string())
    }
}

const WINDOWS_FLUTTER_EXTENSIONS: [&str; 3] = [".bat", ".cmd", ".exe"];

fn env_var_case_insensitive(env: Option<&HashMap<String, String>>, name: &str) -> Option<String> {
    if let Some(env) = env {
        if let Some(val) = env.get(name) {
            return Some(val.clone());
        }
        if let Some((_, val)) = env.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
            return Some(val.clone());
        }
    }
    std::env::var(name).ok()
}

fn env_var_os_case_insensitive(
    env: Option<&HashMap<String, String>>,
    name: &str,
) -> Option<OsString> {
    if let Some(env) = env {
        if let Some(val) = env.get(name) {
            return Some(OsString::from(val));
        }
        if let Some((_, val)) = env.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
            return Some(OsString::from(val));
        }
    }
    std::env::var_os(name)
}

/// Ordered candidate extensions for `flutter` on Windows, respecting `PATHEXT`
/// among the directly executable / batch extensions (`.bat`, `.cmd`, `.exe`).
fn windows_flutter_extensions(pathext: Option<&str>) -> Vec<&'static str> {
    let mut ordered = Vec::with_capacity(WINDOWS_FLUTTER_EXTENSIONS.len());
    if let Some(pathext) = pathext {
        for raw in pathext.split(';') {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let normalized = if trimmed.starts_with('.') {
                trimmed.to_ascii_lowercase()
            } else {
                format!(".{}", trimmed.to_ascii_lowercase())
            };
            if let Some(&ext) = WINDOWS_FLUTTER_EXTENSIONS
                .iter()
                .find(|&&candidate| candidate == normalized)
                && !ordered.contains(&ext)
            {
                ordered.push(ext);
            }
        }
    }
    for &ext in &WINDOWS_FLUTTER_EXTENSIONS {
        if !ordered.contains(&ext) {
            ordered.push(ext);
        }
    }
    ordered
}

fn find_windows_flutter_in_dir(dir: &Path, extensions: &[&str]) -> Option<PathBuf> {
    for ext in extensions {
        let candidate = dir.join(format!("flutter{}", ext));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Resolves `flutter` on Windows across `.bat`, `.cmd`, and `.exe` via `PATH`
/// and `PATHEXT` (supporting standard Flutter SDK, Scoop, Chocolatey, and
/// custom shims), falling back to `"flutter.bat"` if none is found.
fn resolve_windows_flutter(environment: Option<&HashMap<String, String>>) -> String {
    let extensions =
        windows_flutter_extensions(env_var_case_insensitive(environment, "PATHEXT").as_deref());
    if let Some(path_var) = env_var_os_case_insensitive(environment, "PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            if let Some(found) = find_windows_flutter_in_dir(&dir, &extensions) {
                return found.to_string_lossy().to_string();
            }
        }
    }
    "flutter.bat".to_string()
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

    #[test]
    fn windows_extensions_respect_pathext_and_append_defaults() {
        assert_eq!(
            windows_flutter_extensions(Some(".COM;.EXE;.BAT;.CMD;.VBS;.PS1")),
            vec![".exe", ".bat", ".cmd"]
        );
        assert_eq!(
            windows_flutter_extensions(Some("CMD;.bat")),
            vec![".cmd", ".bat", ".exe"]
        );
        assert_eq!(
            windows_flutter_extensions(Some("")),
            vec![".bat", ".cmd", ".exe"]
        );
        assert_eq!(
            windows_flutter_extensions(None),
            vec![".bat", ".cmd", ".exe"]
        );
    }

    #[test]
    fn resolve_windows_flutter_finds_cmd_exe_and_bat_shims_on_path() {
        let dir_empty = tempdir().expect("tempdir");
        let dir_scoop = tempdir().expect("tempdir");
        let dir_choco = tempdir().expect("tempdir");
        let dir_sdk = tempdir().expect("tempdir");

        // Extensionless POSIX script should be ignored on Windows.
        fs::write(dir_empty.path().join("flutter"), "#!/bin/sh\n").expect("write");
        let scoop_cmd = dir_scoop.path().join("flutter.cmd");
        fs::write(&scoop_cmd, "@echo off\r\n").expect("write");
        let choco_exe = dir_choco.path().join("flutter.exe");
        fs::write(&choco_exe, b"MZ").expect("write");
        let sdk_bat = dir_sdk.path().join("flutter.bat");
        fs::write(&sdk_bat, "@echo off\r\n").expect("write");

        // Finds `.cmd` (Scoop shim), skipping extensionless `flutter` in earlier dir.
        let path_val = std::env::join_paths([dir_empty.path(), dir_scoop.path(), dir_sdk.path()])
            .expect("join_paths")
            .to_string_lossy()
            .into_owned();
        let mut env = HashMap::new();
        env.insert("Path".to_string(), path_val);
        env.insert("PathExt".to_string(), ".COM;.EXE;.BAT;.CMD".to_string());
        assert_eq!(
            PathBuf::from(resolve_windows_flutter(Some(&env))),
            scoop_cmd
        );

        // Finds `.exe` (Chocolatey shim).
        let path_val = std::env::join_paths([dir_choco.path(), dir_sdk.path()])
            .expect("join_paths")
            .to_string_lossy()
            .into_owned();
        env.insert("Path".to_string(), path_val);
        assert_eq!(
            PathBuf::from(resolve_windows_flutter(Some(&env))),
            choco_exe
        );

        // Finds `.bat` (standard Flutter SDK).
        let path_val = std::env::join_paths([dir_sdk.path()])
            .expect("join_paths")
            .to_string_lossy()
            .into_owned();
        env.insert("Path".to_string(), path_val);
        assert_eq!(PathBuf::from(resolve_windows_flutter(Some(&env))), sdk_bat);

        // Respects PATHEXT ordering when multiple candidates exist in the same directory.
        let dir_both = tempdir().expect("tempdir");
        let both_bat = dir_both.path().join("flutter.bat");
        let both_cmd = dir_both.path().join("flutter.cmd");
        fs::write(&both_bat, "@echo off\r\n").expect("write");
        fs::write(&both_cmd, "@echo off\r\n").expect("write");
        let path_val = std::env::join_paths([dir_both.path()])
            .expect("join_paths")
            .to_string_lossy()
            .into_owned();
        env.insert("Path".to_string(), path_val);
        env.insert("PathExt".to_string(), ".CMD;.BAT;.EXE".to_string());
        assert_eq!(PathBuf::from(resolve_windows_flutter(Some(&env))), both_cmd);

        // Falls back to "flutter.bat" when nothing matches on PATH.
        env.insert(
            "Path".to_string(),
            dir_empty.path().to_string_lossy().into_owned(),
        );
        assert_eq!(resolve_windows_flutter(Some(&env)), "flutter.bat");
    }

    #[cfg(windows)]
    #[test]
    fn windows_flutter_root_and_path_execute_cmd_shim() {
        let dir = tempdir().expect("tempdir");
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("mkdir");
        let cmd_shim = bin_dir.join("flutter.cmd");
        fs::write(&cmd_shim, "@echo off\r\necho shim-ok\r\n").expect("write cmd");

        // Via FLUTTER_ROOT
        let mut env = HashMap::new();
        env.insert(
            "FLUTTER_ROOT".to_string(),
            dir.path().to_string_lossy().into_owned(),
        );
        let command = FlutterCommand::new(Some(&env));
        assert_eq!(
            PathBuf::from(command.resolve_executable().expect("resolve")),
            cmd_shim
        );
        let out = command
            .base_command()
            .expect("base_command")
            .output()
            .expect("run cmd shim");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "shim-ok");

        // Via PATH
        let mut path_env = HashMap::new();
        path_env.insert("PATH".to_string(), bin_dir.to_string_lossy().into_owned());
        let command = FlutterCommand::new(Some(&path_env));
        assert_eq!(
            PathBuf::from(command.resolve_executable().expect("resolve")),
            cmd_shim
        );
        let out = command
            .base_command()
            .expect("base_command")
            .output()
            .expect("run cmd shim via PATH");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "shim-ok");
    }
}
