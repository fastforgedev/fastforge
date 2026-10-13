//! Package repositories kept in a local directory, ready to be served by
//! any static file host: an APT repository for `.deb` packages and a pacman
//! repository for `.pkg.tar.zst` packages.

use crate::common::{argument, argument_or_env, artifact_path, file_name};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const ENV_APT_REPO_DIR: &str = "APT_REPO_DIR";
const ENV_APT_SIGNING_KEY: &str = "APT_SIGNING_KEY";
const ENV_PACMAN_REPO_DIR: &str = "PACMAN_REPO_DIR";
const ENV_PACMAN_SIGNING_KEY: &str = "PACMAN_SIGNING_KEY";

fn repo_dir(config: &PublishConfig, env: &str) -> Result<PathBuf, PublishError> {
    argument_or_env(config, &["path", "repo-dir"], &[env])
        .filter(|dir| !dir.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            PublishError::General(format!(
                "Repository directory is required. Please provide it via the `path` argument or `{env}` environment variable."
            ))
        })
}

fn run(cmd: &mut Command, what: &str) -> Result<String, PublishError> {
    let output = cmd
        .output()
        .map_err(|e| PublishError::CommandFailed(format!("{what}: {e}")))?;
    if !output.status.success() {
        return Err(PublishError::CommandFailed(format!(
            "{what} failed: {}{}",
            String::from_utf8_lossy(&output.stderr).trim(),
            String::from_utf8_lossy(&output.stdout).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn sha256_file(path: &Path) -> Result<String, PublishError> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

/// `true` or `1`.
fn flag(config: &PublishConfig, key: &str) -> bool {
    argument(config, &[key]).is_some_and(|v| v == "true" || v == "1")
}

// ── APT ─────────────────────────────────────────────────────────────────────

/// Adds a `.deb` to an APT repository in a directory, laid out as
/// `pool/<component>/<p>/<package>/` and `dists/<suite>/`, and regenerates
/// its `Packages` indexes and `Release` file (signed with gpg when a key is
/// given). Requires `dpkg-deb`, and `gpg` for signing.
pub struct AptRepoPublisher;

#[derive(Debug, PartialEq, Eq)]
struct AptRepo {
    root: PathBuf,
    suite: String,
    component: String,
    origin: Option<String>,
    label: Option<String>,
    signing_key: Option<String>,
}

impl AptRepo {
    fn parse(config: &PublishConfig) -> Result<Self, PublishError> {
        let text = |keys: &[&str]| argument(config, keys).filter(|v| !v.trim().is_empty());
        Ok(Self {
            root: repo_dir(config, ENV_APT_REPO_DIR)?,
            suite: text(&["suite"]).unwrap_or_else(|| "stable".to_string()),
            component: text(&["component"]).unwrap_or_else(|| "main".to_string()),
            origin: text(&["origin"]),
            label: text(&["label"]),
            signing_key: argument_or_env(config, &["signing-key"], &[ENV_APT_SIGNING_KEY])
                .filter(|v| !v.trim().is_empty()),
        })
    }

    fn pool(&self) -> PathBuf {
        self.root.join("pool").join(&self.component)
    }

    fn dist(&self) -> PathBuf {
        self.root.join("dists").join(&self.suite)
    }
}

/// The control paragraph of a `.deb`, as `dpkg-deb --field` prints it.
fn deb_control(deb: &Path) -> Result<String, PublishError> {
    run(
        Command::new("dpkg-deb").arg("--field").arg(deb),
        "dpkg-deb --field",
    )
}

/// The value of a single-line control field.
fn control_field<'a>(control: &'a str, name: &str) -> Option<&'a str> {
    control.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

/// A `Packages` stanza: the control paragraph plus where the file is and
/// its size and checksum, as `dpkg-scanpackages` writes it.
fn packages_entry(control: &str, filename: &str, size: u64, sha256: &str) -> String {
    let mut entry = control.trim_end().to_string();
    entry.push_str(&format!(
        "\nFilename: {filename}\nSize: {size}\nSHA256: {sha256}\n"
    ));
    entry
}

/// Every `.deb` under `dir`, sorted.
fn find_debs(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            find_debs(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "deb") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn gzip(data: &[u8]) -> Result<Vec<u8>, PublishError> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

/// Regenerates `dists/<suite>/<component>/binary-<arch>/Packages{,.gz}` from
/// the pool and the `Release` file listing them.
fn write_apt_indexes(repo: &AptRepo) -> Result<Vec<PathBuf>, PublishError> {
    let mut debs = Vec::new();
    find_debs(&repo.pool(), &mut debs)?;
    let mut by_arch: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for deb in &debs {
        let control = deb_control(deb)?;
        let arch = control_field(&control, "Architecture")
            .unwrap_or("all")
            .to_string();
        let entry = packages_entry(
            &control,
            &relative(&repo.root, deb),
            std::fs::metadata(deb)?.len(),
            &sha256_file(deb)?,
        );
        by_arch.entry(arch).or_default().push(entry);
    }
    // `all` packages belong in every architecture's index.
    let all = by_arch.remove("all").unwrap_or_default();
    if by_arch.is_empty() && !all.is_empty() {
        by_arch.insert("amd64".to_string(), Vec::new());
    }

    let dist = repo.dist();
    let _ = std::fs::remove_dir_all(dist.join(&repo.component));
    let mut index_files = Vec::new();
    for (arch, entries) in &by_arch {
        let dir = dist.join(&repo.component).join(format!("binary-{arch}"));
        std::fs::create_dir_all(&dir)?;
        let packages = entries
            .iter()
            .chain(all.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.join("Packages"), &packages)?;
        std::fs::write(dir.join("Packages.gz"), gzip(packages.as_bytes())?)?;
        index_files.push(dir.join("Packages"));
        index_files.push(dir.join("Packages.gz"));
    }

    let architectures: BTreeSet<&str> = by_arch.keys().map(String::as_str).collect();
    let mut release = String::new();
    if let Some(origin) = &repo.origin {
        release.push_str(&format!("Origin: {origin}\n"));
    }
    if let Some(label) = &repo.label {
        release.push_str(&format!("Label: {label}\n"));
    }
    release.push_str(&format!(
        "Suite: {suite}\nCodename: {suite}\nDate: {date}\nArchitectures: {archs}\nComponents: {component}\nSHA256:\n",
        suite = repo.suite,
        date = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S UTC"),
        archs = architectures.into_iter().collect::<Vec<_>>().join(" "),
        component = repo.component,
    ));
    for file in &index_files {
        release.push_str(&format!(
            " {} {} {}\n",
            sha256_file(file)?,
            std::fs::metadata(file)?.len(),
            relative(&dist, file)
        ));
    }
    std::fs::write(dist.join("Release"), release)?;
    for stale in ["InRelease", "Release.gpg"] {
        let _ = std::fs::remove_file(dist.join(stale));
    }
    Ok(index_files)
}

fn sign_apt_release(repo: &AptRepo, key: &str) -> Result<(), PublishError> {
    let dist = repo.dist();
    let release = dist.join("Release");
    run(
        Command::new("gpg")
            .args([
                "--batch",
                "--yes",
                "--local-user",
                key,
                "--clearsign",
                "--output",
            ])
            .arg(dist.join("InRelease"))
            .arg(&release),
        "gpg --clearsign",
    )?;
    run(
        Command::new("gpg")
            .args([
                "--batch",
                "--yes",
                "--local-user",
                key,
                "--armor",
                "--detach-sign",
                "--output",
            ])
            .arg(dist.join("Release.gpg"))
            .arg(&release),
        "gpg --detach-sign",
    )?;
    Ok(())
}

impl AppPublisher for AptRepoPublisher {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        "apt-repo"
    }

    fn is_supported_on_current_platform(&self) -> bool {
        true
    }

    fn perform_publish(
        &self,
        config: &PublishConfig,
        _on_progress: Option<&PublishProgressCallback>,
    ) -> Result<PublishResult, PublishError> {
        let artifact = PathBuf::from(artifact_path(config)?);
        if artifact.extension().is_none_or(|ext| ext != "deb") {
            return Err(PublishError::General(format!(
                "apt-repo publishes `.deb` packages, not `{}`",
                artifact.display()
            )));
        }
        let repo = AptRepo::parse(config)?;
        let control = deb_control(&artifact)?;
        let package = control_field(&control, "Package")
            .ok_or_else(|| PublishError::General("The .deb has no Package field".into()))?;
        let initial = package
            .strip_prefix("lib")
            .and_then(|rest| rest.get(..1).map(|c| format!("lib{c}")))
            .unwrap_or_else(|| package.get(..1).unwrap_or("_").to_string());
        let dest_dir = repo.pool().join(initial).join(package);
        std::fs::create_dir_all(&dest_dir)?;
        let dest = dest_dir.join(file_name(&artifact.display().to_string())?);
        if flag(config, "replace") || !dest.exists() {
            std::fs::copy(&artifact, &dest)?;
        } else if sha256_file(&dest)? != sha256_file(&artifact)? {
            return Err(PublishError::General(format!(
                "{} is already in the repository with different contents; publish a new version or pass replace=true",
                relative(&repo.root, &dest)
            )));
        }

        write_apt_indexes(&repo)?;
        if let Some(key) = &repo.signing_key {
            sign_apt_release(&repo, key)?;
        }
        Ok(PublishResult {
            success: true,
            message: dest.display().to_string(),
        })
    }
}

// ── pacman ──────────────────────────────────────────────────────────────────

/// Adds a pacman package to a repository in a directory with `repo-add`,
/// which keeps `<repo>.db` and `<repo>.files` up to date. Requires
/// `repo-add` (pacman), and `gpg` for signing.
pub struct PacmanRepoPublisher;

impl AppPublisher for PacmanRepoPublisher {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        "pacman-repo"
    }

    fn is_supported_on_current_platform(&self) -> bool {
        true
    }

    fn perform_publish(
        &self,
        config: &PublishConfig,
        _on_progress: Option<&PublishProgressCallback>,
    ) -> Result<PublishResult, PublishError> {
        let artifact = PathBuf::from(artifact_path(config)?);
        let name = file_name(&artifact.display().to_string())?;
        if !name.contains(".pkg.tar.") {
            return Err(PublishError::General(format!(
                "pacman-repo publishes `.pkg.tar.*` packages, not `{name}`"
            )));
        }
        let root = repo_dir(config, ENV_PACMAN_REPO_DIR)?;
        let repo_name = argument(config, &["repo", "repo-name"])
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| {
                PublishError::General(
                    "The repository name is required (the `repo` argument), as in `[repo]` of pacman.conf.".into(),
                )
            })?;
        let signing_key = argument_or_env(config, &["signing-key"], &[ENV_PACMAN_SIGNING_KEY])
            .filter(|v| !v.trim().is_empty());
        std::fs::create_dir_all(&root)?;
        let dest = root.join(&name);
        std::fs::copy(&artifact, &dest)?;

        let mut cmd = Command::new("repo-add");
        cmd.arg("--remove");
        if let Some(key) = &signing_key {
            run(
                Command::new("gpg")
                    .args([
                        "--batch",
                        "--yes",
                        "--local-user",
                        key,
                        "--detach-sign",
                        "--output",
                    ])
                    .arg(root.join(format!("{name}.sig")))
                    .arg(&dest),
                "gpg --detach-sign",
            )?;
            cmd.args(["--sign", "--key", key]);
        }
        cmd.arg(root.join(format!("{repo_name}.db.tar.gz")))
            .arg(&dest);
        run(&mut cmd, "repo-add")?;
        Ok(PublishResult {
            success: true,
            message: dest.display().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config_with(arguments: &[(&str, &str)]) -> PublishConfig {
        PublishConfig {
            publish_arguments: Some(
                arguments
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect::<HashMap<_, _>>(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn apt_repo_defaults_to_stable_main() {
        let repo = AptRepo::parse(&config_with(&[("path", "/srv/apt")])).unwrap();
        assert_eq!(repo.pool(), Path::new("/srv/apt/pool/main"));
        assert_eq!(repo.dist(), Path::new("/srv/apt/dists/stable"));
        assert!(AptRepo::parse(&config_with(&[])).is_err());
    }

    #[test]
    fn packages_entry_appends_location_and_checksum() {
        let control =
            "Package: hello\nVersion: 1.0.0\nArchitecture: amd64\nDescription: Hi\n there\n";
        let entry = packages_entry(control, "pool/main/h/hello/hello.deb", 42, "abc");
        assert_eq!(
            entry,
            "Package: hello\nVersion: 1.0.0\nArchitecture: amd64\nDescription: Hi\n there\nFilename: pool/main/h/hello/hello.deb\nSize: 42\nSHA256: abc\n"
        );
        assert_eq!(control_field(control, "architecture"), Some("amd64"));
    }
}
