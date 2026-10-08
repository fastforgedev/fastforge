//! `fastforge upgrade`: replaces the running binary with the latest release.
//!
//! The Dart CLI upgraded itself with `dart pub global activate fastforge`.
//! The Rust binary is installed from the GitHub release archives by
//! `install.sh` / `install.ps1`, so this downloads the same archive
//! (`fastforge-<version>-<target>.tar.gz|zip`) and swaps the executable in
//! place.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use clap::Args;

use super::version_check::{
    CURRENT_VERSION, ReleaseAsset, Spinner, archive_name, fetch_latest_release, http_client,
    is_newer, release_target,
};

#[derive(Args)]
pub struct UpgradeArgs {
    /// Reinstall the latest release even if the current version is up to date.
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "fastforge.exe"
    } else {
        "fastforge"
    }
}

pub(crate) async fn download(
    client: &reqwest::Client,
    asset: &ReleaseAsset,
    dest: &Path,
) -> Result<()> {
    let mut response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .with_context(|| format!("failed to download {}", asset.browser_download_url))?
        .error_for_status()
        .with_context(|| format!("failed to download {}", asset.browser_download_url))?;
    let total = response.content_length().unwrap_or(asset.size);
    let show_progress = std::io::stderr().is_terminal() && total > 0;
    let mut file = std::fs::File::create(dest)
        .with_context(|| format!("failed to create {}", dest.display()))?;
    let mut sent: u64 = 0;
    while let Some(chunk) = response.chunk().await.context("download interrupted")? {
        file.write_all(&chunk)?;
        sent += chunk.len() as u64;
        if show_progress {
            let percentage = (sent * 100 / total).min(100);
            let filled = (percentage / 5) as usize;
            eprint!(
                "\rDownloading {}: {}{} {}/{} {}%",
                asset.name,
                "█".repeat(filled),
                "░".repeat(20 - filled),
                sent,
                total,
                percentage
            );
        }
    }
    if show_progress {
        eprintln!();
    }
    file.flush()?;
    Ok(())
}

pub(crate) fn extract(archive: &Path, into: &Path) -> Result<()> {
    // `tar` ships with macOS, Linux and Windows 10+ (bsdtar, which also
    // understands zip archives).
    let flags = if cfg!(windows) { "-xf" } else { "-xzf" };
    let status = Command::new("tar")
        .arg(flags)
        .arg(archive)
        .arg("-C")
        .arg(into)
        .status()
        .context("failed to run `tar` to extract the release archive")?;
    if !status.success() {
        bail!("failed to extract {}", archive.display());
    }
    Ok(())
}

fn permission_hint(err: &std::io::Error, dir: &Path) -> anyhow::Error {
    if err.kind() == std::io::ErrorKind::PermissionDenied {
        let hint = if cfg!(windows) {
            "Re-run from an elevated terminal".to_string()
        } else {
            "Re-run with elevated permissions (e.g. `sudo fastforge upgrade`)".to_string()
        };
        anyhow!(
            "Permission denied writing to {}. {hint}, or reinstall with the script at https://fastforge.dev.",
            dir.display()
        )
    } else {
        anyhow!(
            "failed to replace the fastforge binary in {}: {err}",
            dir.display()
        )
    }
}

/// Moves `new_binary` over `current_exe`. On Unix a rename over the running
/// executable is atomic and safe; on Windows the running executable can only
/// be renamed out of the way, so it is parked as `<exe>.old` first.
fn replace_executable(new_binary: &Path, current_exe: &Path) -> Result<()> {
    let dir = current_exe.parent().ok_or_else(|| {
        anyhow!(
            "cannot determine the directory of {}",
            current_exe.display()
        )
    })?;
    let staged = dir.join(format!(".{}.upgrade-{}", binary_name(), std::process::id()));
    std::fs::copy(new_binary, &staged).map_err(|e| permission_hint(&e, dir))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
        if let Err(e) = std::fs::rename(&staged, current_exe) {
            let _ = std::fs::remove_file(&staged);
            return Err(permission_hint(&e, dir));
        }
    }

    #[cfg(windows)]
    {
        let parked = current_exe.with_extension("exe.old");
        let _ = std::fs::remove_file(&parked);
        if let Err(e) = std::fs::rename(current_exe, &parked) {
            let _ = std::fs::remove_file(&staged);
            return Err(permission_hint(&e, dir));
        }
        if let Err(e) = std::fs::rename(&staged, current_exe) {
            let _ = std::fs::rename(&parked, current_exe);
            let _ = std::fs::remove_file(&staged);
            return Err(permission_hint(&e, dir));
        }
    }

    Ok(())
}

fn resolve_current_exe() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("cannot locate the running fastforge binary")?;
    Ok(std::fs::canonicalize(&exe).unwrap_or(exe))
}

pub async fn execute(args: &UpgradeArgs) -> Result<()> {
    let target = release_target()?;
    let client = http_client(Some(Duration::from_secs(600)))?;

    let spinner = Spinner::start("Checking for updates...");
    let release = fetch_latest_release(&client).await;
    spinner.stop();
    let Some(release) = release? else {
        bail!("No published fastforge release provides a prebuilt binary for {target}.");
    };
    let latest = release.version().to_string();

    if !args.force && !is_newer(&latest, CURRENT_VERSION) {
        eprintln!("\x1b[92m✓\x1b[0m Fastforge is already up to date ({CURRENT_VERSION}).");
        return Ok(());
    }

    let archive_name = archive_name(&latest, target);
    let asset = release
        .platform_asset()
        .ok_or_else(|| anyhow!("Release v{latest} has no asset named {archive_name}"))?;

    let work_dir = std::env::temp_dir().join(format!("fastforge-upgrade-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(&work_dir)
        .with_context(|| format!("failed to create {}", work_dir.display()))?;

    let result = async {
        let archive_path = work_dir.join(&archive_name);
        download(&client, asset, &archive_path).await?;
        extract(&archive_path, &work_dir)?;
        let new_binary = work_dir
            .join(format!("fastforge-{latest}-{target}"))
            .join(binary_name());
        if !new_binary.is_file() {
            bail!(
                "Binary not found in archive at expected path: {}",
                new_binary.display()
            );
        }
        let current_exe = resolve_current_exe()?;
        replace_executable(&new_binary, &current_exe)?;
        Ok(current_exe)
    }
    .await;
    let _ = std::fs::remove_dir_all(&work_dir);
    let current_exe = result?;

    eprintln!(
        "\x1b[92m✓\x1b[0m Upgraded Fastforge {CURRENT_VERSION} → \x1b[1;92m{latest}\x1b[0m ({})",
        current_exe.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_executable_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join(binary_name());
        std::fs::write(&current, b"old").unwrap();
        let new_binary = dir.path().join("new-binary");
        std::fs::write(&new_binary, b"new").unwrap();

        replace_executable(&new_binary, &current).unwrap();

        assert_eq!(std::fs::read(&current).unwrap(), b"new");
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".upgrade-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "staged file left behind: {leftovers:?}"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&current).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
    }

    #[test]
    fn extracts_release_archive_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("fastforge-9.9.9-test");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(binary_name()), b"bin").unwrap();
        let archive = dir.path().join("a.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(dir.path())
            .arg("fastforge-9.9.9-test")
            .status()
            .unwrap();
        assert!(status.success());
        let out = dir.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        if cfg!(unix) {
            extract(&archive, &out).unwrap();
            assert!(
                out.join("fastforge-9.9.9-test")
                    .join(binary_name())
                    .is_file()
            );
        }
    }
}
