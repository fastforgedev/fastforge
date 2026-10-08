//! `fastforge run`: run the app on a device, like `flutter run`.

use anyhow::{Context, Result, anyhow, bail};
use clap::Args;
use fastforge_app_builder::FlutterCommand;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;

use super::package::is_flutter_project;
use crate::utils::bright_black;

#[derive(Args)]
pub struct RunArgs {
    /// Platform to run on; picks a matching device when --device-id is omitted.
    #[arg(
        short,
        long = "platform",
        value_name = "android,ios,linux,macos,ohos,windows,web"
    )]
    pub platform: Option<String>,
    /// Target device id or name (see `flutter devices`).
    #[arg(short = 'd', long = "device-id", value_name = "ID")]
    pub device_id: Option<String>,
    /// Run a release build.
    #[arg(long = "release", conflicts_with = "profile")]
    pub release: bool,
    /// Run a profile build.
    #[arg(long = "profile")]
    pub profile: bool,
    /// Build flavor.
    #[arg(long = "flavor")]
    pub flavor: Option<String>,
    /// Main entry-point file of the app (default lib/main.dart).
    #[arg(short = 't', long = "target", value_name = "PATH")]
    pub target: Option<String>,
    /// Compile-time variable; may be repeated.
    #[arg(long = "dart-define", value_name = "KEY=VALUE")]
    pub dart_define: Vec<String>,
    /// File of compile-time variables (JSON or .env).
    #[arg(long = "dart-define-from-file", value_name = "PATH")]
    pub dart_define_from_file: Option<String>,
    /// Run on a remote host from `~/.fastforge/hosts.yaml`; hot reload syncs
    /// the project first. `auto` picks a host listing the platform.
    #[arg(long = "host", value_name = "NAME|auto")]
    pub host: Option<String>,
    /// With --host: once the app runs, show its window on this machine
    /// (Windows hosts only for now).
    #[arg(long = "remote-window", requires = "host")]
    pub remote_window: bool,
    /// Extra arguments passed to `flutter run`.
    #[arg(last = true, value_name = "FLUTTER_ARGS")]
    pub flutter_args: Vec<String>,
}

/// One entry of `flutter devices --machine`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub name: String,
    pub id: String,
    #[serde(default)]
    pub is_supported: bool,
    #[serde(default)]
    pub target_platform: String,
}

/// `targetPlatform` prefix of `flutter devices --machine` for a platform.
fn target_platform_prefix(platform: &str) -> Result<&'static str> {
    Ok(match platform {
        "android" => "android",
        "ios" => "ios",
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        "web" => "web",
        "ohos" => "ohos",
        other => bail!("Unsupported platform `{other}`"),
    })
}

/// Picks the device for `platform`: the first supported matching device,
/// preferring Chrome for the web.
pub fn select_device<'a>(platform: &str, devices: &'a [Device]) -> Result<&'a Device> {
    let prefix = target_platform_prefix(platform)?;
    let matching: Vec<&Device> = devices
        .iter()
        .filter(|d| d.is_supported && d.target_platform.starts_with(prefix))
        .collect();
    if platform == "web"
        && let Some(chrome) = matching.iter().find(|d| d.id == "chrome")
    {
        return Ok(chrome);
    }
    matching.first().copied().ok_or_else(|| {
        let available: Vec<String> = devices
            .iter()
            .filter(|d| d.is_supported)
            .map(|d| format!("{} ({})", d.name, d.id))
            .collect();
        anyhow!(
            "No `{platform}` device found. Available devices: {}",
            if available.is_empty() {
                "none".to_string()
            } else {
                available.join(", ")
            }
        )
    })
}

/// Parses `flutter devices --machine`, which may print notices before the JSON.
fn parse_devices(output: &str) -> Result<Vec<Device>> {
    let start = output
        .find('[')
        .ok_or_else(|| anyhow!("Unexpected `flutter devices --machine` output"))?;
    serde_json::from_str(&output[start..]).context("Failed to parse `flutter devices --machine`")
}

impl RunArgs {
    /// Options shared by `flutter run`, `flutter build` and (without the
    /// flavor, which it doesn't accept) `flutter attach`.
    pub fn compile_args(&self, with_flavor: bool) -> Vec<String> {
        let mut args = Vec::new();
        if with_flavor && let Some(flavor) = &self.flavor {
            args.extend(["--flavor".to_string(), flavor.clone()]);
        }
        if let Some(target) = &self.target {
            args.extend(["--target".to_string(), target.clone()]);
        }
        for define in &self.dart_define {
            args.push(format!("--dart-define={define}"));
        }
        if let Some(file) = &self.dart_define_from_file {
            args.push(format!("--dart-define-from-file={file}"));
        }
        args
    }

    /// Arguments of `flutter run`, given the resolved device.
    pub fn flutter_run_args(&self, device: Option<&str>) -> Vec<String> {
        let mut args = vec!["run".to_string()];
        if let Some(device) = device {
            args.extend(["-d".to_string(), device.to_string()]);
        }
        if self.release {
            args.push("--release".into());
        }
        if self.profile {
            args.push("--profile".into());
        }
        args.extend(self.compile_args(true));
        args.extend(self.flutter_args.iter().cloned());
        args
    }
}

pub async fn execute(args: &RunArgs) -> Result<()> {
    if let Some(host) = args.host.as_deref() {
        return super::remote::run(args, host).await;
    }
    if !is_flutter_project() {
        bail!("No pubspec.yaml found: `fastforge run` currently supports Flutter projects only.");
    }

    let env: HashMap<String, String> = std::env::vars().collect();
    let flutter = FlutterCommand::new(Some(&env));
    let device = match (&args.device_id, &args.platform) {
        (Some(id), _) => Some(id.clone()),
        (None, Some(platform)) => {
            let mut command = flutter.base_command().map_err(|e| anyhow!("{e}"))?;
            let output = command
                .args(["devices", "--machine"])
                .stderr(Stdio::inherit())
                .output()
                .context("Failed to run `flutter devices`")?;
            let devices = parse_devices(&String::from_utf8_lossy(&output.stdout))?;
            let device = select_device(platform, &devices)?;
            eprintln!(
                "Using device {} ({}); pass -d to choose another.",
                device.name, device.id
            );
            Some(device.id.clone())
        }
        (None, None) => None,
    };

    if device.as_deref() == Some("windows") && super::run_desktop::needs_desktop_session() {
        return super::run_desktop::run(args, &flutter).await;
    }

    let run_args = args.flutter_run_args(device.as_deref());
    eprintln!(
        "{}",
        bright_black(&format!("$ flutter {}", run_args.join(" ")))
    );
    let mut command = flutter.base_command().map_err(|e| anyhow!("{e}"))?;
    if let Some((key, value)) = super::run_desktop::linux_display_env(device.as_deref()) {
        eprintln!("This session has no display; showing the app on the desktop ({key}={value}).");
        command.env(key, value);
    }
    let mut child = command
        .args(&run_args)
        .spawn()
        .context("Failed to start `flutter run`")?;
    // Ctrl-C belongs to flutter (it shuts the app down); don't die before it.
    let guard = tokio::spawn(async { while tokio::signal::ctrl_c().await.is_ok() {} });
    let status = tokio::task::spawn_blocking(move || child.wait()).await??;
    guard.abort();
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        args: RunArgs,
    }

    const DEVICES: &str = r#"Some notice
[
  {"name": "Pixel", "id": "emulator-5554", "isSupported": true, "targetPlatform": "android-arm64"},
  {"name": "macOS", "id": "macos", "isSupported": true, "targetPlatform": "darwin"},
  {"name": "Web Server", "id": "web-server", "isSupported": true, "targetPlatform": "web-javascript"},
  {"name": "Chrome", "id": "chrome", "isSupported": true, "targetPlatform": "web-javascript"},
  {"name": "Old iPhone", "id": "abc", "isSupported": false, "targetPlatform": "ios"}
]"#;

    #[test]
    fn selects_devices_by_platform() {
        let devices = parse_devices(DEVICES).unwrap();
        assert_eq!(
            select_device("android", &devices).unwrap().id,
            "emulator-5554"
        );
        assert_eq!(select_device("macos", &devices).unwrap().id, "macos");
        assert_eq!(select_device("web", &devices).unwrap().id, "chrome");
        let error = select_device("ios", &devices).unwrap_err().to_string();
        assert!(error.contains("No `ios` device found"), "{error}");
        assert!(error.contains("Pixel (emulator-5554)"), "{error}");
        assert!(select_device("tvos", &devices).is_err());
    }

    #[test]
    fn builds_flutter_run_arguments() {
        let cli = Cli::parse_from([
            "x",
            "--release",
            "--flavor",
            "prod",
            "-t",
            "lib/main_prod.dart",
            "--dart-define",
            "A=1",
            "--dart-define-from-file",
            "env.json",
            "--",
            "--verbose",
        ]);
        assert_eq!(
            cli.args.flutter_run_args(Some("macos")),
            vec![
                "run",
                "-d",
                "macos",
                "--release",
                "--flavor",
                "prod",
                "--target",
                "lib/main_prod.dart",
                "--dart-define=A=1",
                "--dart-define-from-file=env.json",
                "--verbose",
            ]
        );
        assert!(Cli::try_parse_from(["x", "--release", "--profile"]).is_err());
        assert!(Cli::try_parse_from(["x", "--remote-window"]).is_err());
        let cli = Cli::parse_from(["x", "--host", "win", "--remote-window"]);
        assert!(cli.args.remote_window);
    }
}
