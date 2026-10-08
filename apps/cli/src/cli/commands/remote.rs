//! Running commands on a remote host (`--host`).
//!
//! The project is synced to the host, the same fastforge command runs there
//! through `fastforge remote-agent exec`, and its artifacts are fetched back
//! into the local output directory.

use anyhow::{Result, anyhow, bail};
use fastforge_app_builder::Platform;
use fastforge_remote::interactive::{LineHook, ResyncTarget};
use fastforge_remote::{HostConfig, HostsFile, LocalProject, RemoteClient, RunSpec, new_run_id};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::package::{PackageArgs, resolve_output};
use super::platform_infer;
use super::remote_window::RemoteWindow;
use super::run::RunArgs;
use crate::config::DistributeOptions;
use crate::utils::{bright_green, yellow};

/// Flags whose value is a local path that must be valid on the host too.
const PACKAGE_PATH_FLAGS: &[&str] = &["--build-export-options-plist", "--build-target"];

/// Path flags of `run`.
const RUN_PATH_FLAGS: &[&str] = &["-t", "--target", "--dart-define-from-file"];

/// Flags the client handles itself and never forwards.
const LOCAL_ONLY_FLAGS: &[&str] = &["--host", "--output"];

/// Switches (no value) the client handles itself and never forwards.
const LOCAL_ONLY_SWITCHES: &[&str] = &["--remote-window"];

/// Resolves `--host <name|auto>`; `auto` picks the first host whose
/// `platforms` contains `platform`.
pub fn resolve_host(spec: &str, platform: Option<&str>, hosts: &HostsFile) -> Result<HostConfig> {
    if spec != "auto" {
        return hosts.require(spec).cloned();
    }
    let platform = platform
        .ok_or_else(|| anyhow!("`--host auto` needs the platform; pass --platform explicitly"))?;
    hosts.find_for_platform(platform).cloned().ok_or_else(|| {
        anyhow!(
            "No remote host builds `{platform}`: list it in a host's `platforms` \
             (`fastforge host doctor <name>` detects them)"
        )
    })
}

/// Prints a hint when `platform` can't be built here but a configured host
/// can build it.
pub fn hint_if_unsupported(platform: &str) {
    let Ok(parsed) = platform.parse::<Platform>() else {
        return;
    };
    if platform_infer::buildable_on_host(parsed, Platform::current()) {
        return;
    }
    let Ok(hosts) = HostsFile::load() else {
        return;
    };
    if let Some(host) = hosts.find_for_platform(platform) {
        let host = &host.name;
        eprintln!(
            "{}",
            yellow(&format!(
                "Hint: `{platform}` can't be built on this machine; add `--host auto` to build it on `{host}`."
            ))
        );
    }
}

/// The arguments the host runs: everything after `subcommand` in `raw`,
/// minus the local-only flags, with path values made relative to the working
/// directory and checked to lie inside the synced project.
pub fn remote_argv(
    raw: &[String],
    subcommand: &str,
    path_flags: &[&str],
    project: &LocalProject,
) -> Result<Vec<String>> {
    let start = raw
        .iter()
        .skip(1)
        .position(|a| a == subcommand)
        .map(|i| i + 2)
        .ok_or_else(|| anyhow!("`{subcommand}` not found in the command line"))?;
    let mut out = vec![subcommand.to_string()];
    let mut iter = raw[start..].iter();
    while let Some(arg) = iter.next() {
        // Everything after `--` belongs to the wrapped tool.
        if arg == "--" {
            out.push(arg.clone());
            out.extend(iter.cloned());
            break;
        }
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        if LOCAL_ONLY_SWITCHES.contains(&arg.as_str()) {
            continue;
        }
        if LOCAL_ONLY_FLAGS.contains(&flag) {
            if inline.is_none() {
                iter.next();
            }
            continue;
        }
        if path_flags.contains(&flag) {
            let value = match inline {
                Some(value) => value,
                None => iter
                    .next()
                    .cloned()
                    .ok_or_else(|| anyhow!("{flag} needs a value"))?,
            };
            out.push(flag.to_string());
            out.push(remote_path(&value, project)?);
            continue;
        }
        out.push(arg.clone());
    }
    Ok(out)
}

/// `value` as a path relative to the working directory on the host.
fn remote_path(value: &str, project: &LocalProject) -> Result<String> {
    let path = Path::new(value);
    let key = project.key_for(path).ok_or_else(|| {
        anyhow!(
            "`{value}` is outside {} and isn't synced to the remote host",
            project.root.display()
        )
    })?;
    if !path.is_absolute() {
        return Ok(value.to_string());
    }
    // Absolute local paths don't exist remotely: go up from the cwd.
    let depth = project.cwd.split('/').filter(|s| !s.is_empty()).count();
    let mut rel = "../".repeat(depth);
    rel.push_str(&key);
    Ok(if rel.is_empty() { ".".into() } else { rel })
}

/// `fastforge package --host`.
pub fn package(args: &PackageArgs, host_spec: &str, targets: &[&str]) -> Result<()> {
    let options = DistributeOptions::load()?;
    let hosts = HostsFile::load()?;
    let platform = match (&args.platform, host_spec) {
        (Some(p), _) => Some(p.clone()),
        (None, "auto") => Some(
            platform_infer::infer_platform_anywhere(targets)?
                .as_str()
                .to_string(),
        ),
        // A named host infers the platform itself, against its own OS.
        (None, _) => None,
    };
    let host = resolve_host(host_spec, platform.as_deref(), &hosts)?;

    let cwd = std::env::current_dir()?;
    let project = LocalProject::detect(&cwd)?;
    let raw: Vec<String> = std::env::args().collect();
    let mut argv = remote_argv(&raw, "package", PACKAGE_PATH_FLAGS, &project)?;
    if args.platform.is_none()
        && let Some(platform) = &platform
    {
        argv.push("--platform".into());
        argv.push(platform.clone());
    }

    let output = PathBuf::from(resolve_output(args.output.as_deref(), &options));
    let excludes = sync_excludes(&project, args.output.as_deref())?;
    let artifacts = run_remotely(&host, &project, &excludes, argv, &cwd.join(&output))?;

    println!(
        "{}",
        bright_green(&format!(
            "Fetched {} artifact(s) from {}:",
            artifacts.len(),
            host.name
        ))
    );
    for artifact in &artifacts {
        let shown = artifact.strip_prefix(&cwd).unwrap_or(artifact);
        println!("  {}", shown.display());
    }
    Ok(())
}

/// Paths never synced: the local output directory (`--output`, else the
/// default `package` uses), so artifacts aren't uploaded back to the host.
pub fn sync_excludes(project: &LocalProject, output: Option<&str>) -> Result<Vec<String>> {
    let options = DistributeOptions::load()?;
    let output = PathBuf::from(resolve_output(output, &options));
    Ok(project.key_for(&output).into_iter().collect())
}

/// Inserts `extra` before a trailing `-- <args>` section, or at the end.
fn insert_before_passthrough(argv: &mut Vec<String>, extra: Vec<String>) {
    let at = argv.iter().position(|a| a == "--").unwrap_or(argv.len());
    argv.splice(at..at, extra);
}

/// `fastforge run --host`: syncs the project, runs `fastforge run` on the
/// host attached to this terminal, and syncs again before each hot reload.
pub async fn run(args: &RunArgs, host_spec: &str) -> Result<()> {
    let hosts = HostsFile::load()?;
    let platform = match (&args.platform, host_spec) {
        (Some(p), _) => Some(p.clone()),
        (None, "auto") => Some(
            platform_infer::infer_platform_anywhere(&[])?
                .as_str()
                .to_string(),
        ),
        (None, _) => None,
    };
    let host = resolve_host(host_spec, platform.as_deref(), &hosts)?;

    let cwd = std::env::current_dir()?;
    let project = LocalProject::detect(&cwd)?;
    let raw: Vec<String> = std::env::args().collect();
    let mut argv = remote_argv(&raw, "run", RUN_PATH_FLAGS, &project)?;
    let mut extra = Vec::new();
    if args.platform.is_none()
        && let Some(platform) = &platform
    {
        extra.extend(["--platform".to_string(), platform.clone()]);
    }
    if args.device_id.is_none() && platform.as_deref() == Some("web") {
        // No browser to open on the host: serve the app, and the forwarded
        // port makes it reachable from here.
        extra.extend(["--device-id".to_string(), "web-server".to_string()]);
    }
    insert_before_passthrough(&mut argv, extra);

    let mut client = RemoteClient::new(host.clone());
    client.ensure_os()?;
    // Checked now; started once the app runs and stopped when the run ends.
    let window = if args.remote_window {
        Some(Arc::new(
            RemoteWindow::prepare(&client, platform.as_deref()).await?,
        ))
    } else {
        None
    };
    let workspace = client.workspace(&project);
    let run = RunSpec {
        id: new_run_id(),
        cwd: project.cwd.clone(),
        argv,
        env: host.forwarded_env(std::env::vars()),
        env_templates: host.env.clone(),
        append_output: false,
        interactive: true,
    };
    let run_id = run.id.clone();
    eprintln!(
        "Running `fastforge {}` on {} ({})",
        run.argv.join(" "),
        host.name,
        host.destination()
    );
    let excludes = sync_excludes(&project, None)?;
    let (code, _) = client.sync_and_run(&project, &workspace, &excludes, Some(run))?;
    if code != 0 {
        bail!("Syncing to `{}` failed (exit code {code})", host.name);
    }
    let resync = ResyncTarget {
        project,
        workspace: workspace.clone(),
        excludes,
    };
    let on_line = window
        .clone()
        .map(|window| Box::new(move |line: &str, raw: bool| window.observe(line, raw)) as LineHook);
    let code = tokio::task::spawn_blocking(move || {
        client.attach_interactive(&workspace, &run_id, Some(resync), on_line)
    })
    .await??;
    if let Some(window) = &window {
        window.stop();
    }
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// Syncs `project`, runs `argv` on `host` with its output redirected to a run
/// directory, and fetches that directory into `output`.
fn run_remotely(
    host: &HostConfig,
    project: &LocalProject,
    excludes: &[String],
    argv: Vec<String>,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    let mut client = RemoteClient::new(host.clone());
    client.ensure_os()?;
    let workspace = client.workspace(project);
    let run = RunSpec {
        id: new_run_id(),
        cwd: project.cwd.clone(),
        argv,
        env: host.forwarded_env(std::env::vars()),
        env_templates: host.env.clone(),
        append_output: true,
        interactive: false,
    };
    let run_id = run.id.clone();
    eprintln!(
        "Running `fastforge {}` on {} ({})",
        run.argv.join(" "),
        host.name,
        host.destination()
    );
    let (code, _) = client.sync_and_run(project, &workspace, excludes, Some(run))?;
    if code != 0 {
        bail!(
            "The remote run on `{}` failed (exit code {code})",
            host.name
        );
    }
    client.fetch(&workspace, &run_id, output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastforge_remote::TransportKind;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn strips_local_flags_and_maps_paths() {
        let project = LocalProject::new(PathBuf::from("/repo"), Path::new("/repo/app"));
        let raw = strings(&[
            "fastforge",
            "--no-version-check",
            "package",
            "-t",
            "ipa",
            "--host",
            "mac",
            "--output=out/",
            "--build-export-options-plist",
            "/repo/app/ios/Export.plist",
            "--build-target=lib/main_prod.dart",
            "--build-dart-define",
            "A=--host",
        ]);
        let argv = remote_argv(&raw, "package", PACKAGE_PATH_FLAGS, &project).unwrap();
        assert_eq!(
            argv,
            strings(&[
                "package",
                "-t",
                "ipa",
                "--build-export-options-plist",
                "../app/ios/Export.plist",
                "--build-target",
                "lib/main_prod.dart",
                "--build-dart-define",
                "A=--host",
            ])
        );
    }

    #[test]
    fn keeps_passthrough_arguments() {
        let project = LocalProject::new(PathBuf::from("/repo"), Path::new("/repo"));
        let raw = strings(&[
            "fastforge",
            "run",
            "-t",
            "/repo/lib/dev.dart",
            "--host",
            "mac",
            "--remote-window",
            "--",
            "--host",
            "x",
        ]);
        let mut argv = remote_argv(&raw, "run", RUN_PATH_FLAGS, &project).unwrap();
        assert_eq!(
            argv,
            strings(&["run", "-t", "lib/dev.dart", "--", "--host", "x"])
        );
        insert_before_passthrough(&mut argv, strings(&["-p", "web"]));
        assert_eq!(
            argv,
            strings(&[
                "run",
                "-t",
                "lib/dev.dart",
                "-p",
                "web",
                "--",
                "--host",
                "x"
            ])
        );
    }

    #[test]
    fn rejects_paths_outside_the_project() {
        let project = LocalProject::new(PathBuf::from("/repo"), Path::new("/repo"));
        let raw = strings(&[
            "fastforge",
            "package",
            "--build-export-options-plist",
            "/etc/x.plist",
        ]);
        assert!(remote_argv(&raw, "package", PACKAGE_PATH_FLAGS, &project).is_err());
    }

    fn host(name: &str, platforms: &[&str]) -> HostConfig {
        let mut host = HostConfig::new(name);
        host.transport = TransportKind::Local;
        host.platforms = platforms.iter().map(|s| s.to_string()).collect();
        host
    }

    #[test]
    fn resolves_auto_hosts() {
        let mut hosts = HostsFile::default();
        hosts.add(host("mac", &["ios", "macos"]), false).unwrap();
        hosts.add(host("mac2", &["ios"]), false).unwrap();

        assert_eq!(resolve_host("mac2", None, &hosts).unwrap().name, "mac2");
        assert!(resolve_host("nope", None, &hosts).is_err());
        assert!(resolve_host("auto", None, &hosts).is_err());
        assert_eq!(
            resolve_host("auto", Some("ios"), &hosts).unwrap().name,
            "mac"
        );
        assert!(resolve_host("auto", Some("windows"), &hosts).is_err());
    }
}
