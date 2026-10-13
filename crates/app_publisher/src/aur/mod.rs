//! The Arch User Repository: publishes a `-bin` package whose PKGBUILD
//! installs a pacman package Fastforge built, downloaded from where it was
//! published (such as a GitHub release).

use crate::common::{argument, artifact_path, file_name};
use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct AurPublisher;

const PUBLISHER_NAME: &str = "aur";

/// The metadata of a pacman package, from its `.PKGINFO`.
#[derive(Debug, Default, PartialEq, Eq)]
struct PkgInfo {
    pkgname: String,
    /// `pkgver-pkgrel`, as pacman writes it.
    pkgver: String,
    pkgdesc: String,
    url: Option<String>,
    arch: String,
    licenses: Vec<String>,
    depends: Vec<String>,
    optdepends: Vec<String>,
    conflicts: Vec<String>,
    provides: Vec<String>,
    replaces: Vec<String>,
}

impl PkgInfo {
    fn parse(text: &str) -> Self {
        let mut info = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(" = ") else {
                continue;
            };
            let value = value.trim().to_string();
            match key.trim() {
                "pkgname" => info.pkgname = value,
                "pkgver" => info.pkgver = value,
                "pkgdesc" => info.pkgdesc = value,
                "url" => info.url = Some(value),
                "arch" => info.arch = value,
                "license" => info.licenses.push(value),
                "depend" => info.depends.push(value),
                "optdepend" => info.optdepends.push(value),
                "conflict" => info.conflicts.push(value),
                "provides" => info.provides.push(value),
                "replaces" => info.replaces.push(value),
                _ => {}
            }
        }
        info
    }

    /// `(pkgver, pkgrel)`: pacman joins them with the last `-`.
    fn version(&self) -> (&str, &str) {
        self.pkgver
            .rsplit_once('-')
            .unwrap_or((self.pkgver.as_str(), "1"))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct AurPackage {
    name: String,
    maintainer: Option<String>,
    source_url: String,
    source_file: String,
    sha256: String,
    install: Option<String>,
}

/// Quotes a value for a PKGBUILD array or assignment.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

fn array(values: &[String]) -> String {
    values
        .iter()
        .map(|v| quote(v))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The PKGBUILD: it downloads the package and unpacks its files into
/// `$pkgdir`, leaving out pacman's own metadata. The package keeps the
/// dependencies and relations of the original, and provides and conflicts
/// with its name, so `<app>-bin` and `<app>` don't install side by side.
fn pkgbuild(info: &PkgInfo, aur: &AurPackage) -> String {
    let (pkgver, pkgrel) = info.version();
    let mut provides = info.provides.clone();
    let mut conflicts = info.conflicts.clone();
    if aur.name != info.pkgname {
        provides.push(info.pkgname.clone());
        conflicts.push(info.pkgname.clone());
    }
    let mut lines = Vec::new();
    if let Some(maintainer) = &aur.maintainer {
        lines.push(format!("# Maintainer: {maintainer}"));
    }
    lines.push(format!("pkgname={}", aur.name));
    lines.push(format!("pkgver={pkgver}"));
    lines.push(format!("pkgrel={pkgrel}"));
    lines.push(format!("pkgdesc={}", quote(&info.pkgdesc)));
    lines.push(format!("arch=({})", quote(&info.arch)));
    if let Some(url) = &info.url {
        lines.push(format!("url={}", quote(url)));
    }
    lines.push(format!("license=({})", array(&info.licenses)));
    for (key, values) in [
        ("depends", &info.depends),
        ("optdepends", &info.optdepends),
        ("provides", &provides),
        ("conflicts", &conflicts),
        ("replaces", &info.replaces),
    ] {
        if !values.is_empty() {
            lines.push(format!("{key}=({})", array(values)));
        }
    }
    lines.push("options=('!strip' '!debug')".to_string());
    if let Some(install) = &aur.install {
        lines.push(format!("install={install}"));
    }
    lines.push(format!(
        "source=({})",
        quote(&format!("{}::{}", aur.source_file, aur.source_url))
    ));
    lines.push(format!("noextract=({})", quote(&aur.source_file)));
    lines.push(format!("sha256sums=({})", quote(&aur.sha256)));
    lines.push(String::new());
    lines.push("package() {".to_string());
    lines.push(format!(
        "  bsdtar -xf \"$srcdir/{}\" -C \"$pkgdir\" --exclude .PKGINFO --exclude .BUILDINFO --exclude .MTREE --exclude .INSTALL",
        aur.source_file
    ));
    lines.push("}".to_string());
    lines.join("\n") + "\n"
}

/// `.SRCINFO`, in the format `makepkg --printsrcinfo` writes.
fn srcinfo(info: &PkgInfo, aur: &AurPackage) -> String {
    let (pkgver, pkgrel) = info.version();
    let mut provides = info.provides.clone();
    let mut conflicts = info.conflicts.clone();
    if aur.name != info.pkgname {
        provides.push(info.pkgname.clone());
        conflicts.push(info.pkgname.clone());
    }
    let mut lines = vec![
        format!("pkgbase = {}", aur.name),
        format!("\tpkgdesc = {}", info.pkgdesc),
        format!("\tpkgver = {pkgver}"),
        format!("\tpkgrel = {pkgrel}"),
    ];
    if let Some(url) = &info.url {
        lines.push(format!("\turl = {url}"));
    }
    if let Some(install) = &aur.install {
        lines.push(format!("\tinstall = {install}"));
    }
    lines.push(format!("\tarch = {}", info.arch));
    for (key, values) in [
        ("license", &info.licenses),
        ("depends", &info.depends),
        ("optdepends", &info.optdepends),
        ("provides", &provides),
        ("conflicts", &conflicts),
        ("replaces", &info.replaces),
    ] {
        for value in values {
            lines.push(format!("\t{key} = {value}"));
        }
    }
    lines.push("\tnoextract = ".to_string() + &aur.source_file);
    lines.push("\toptions = !strip".to_string());
    lines.push("\toptions = !debug".to_string());
    lines.push(format!(
        "\tsource = {}::{}",
        aur.source_file, aur.source_url
    ));
    lines.push(format!("\tsha256sums = {}", aur.sha256));
    lines.push(String::new());
    lines.push(format!("pkgname = {}", aur.name));
    lines.join("\n") + "\n"
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

fn git(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir);
    cmd
}

/// A scratch directory under the system temp directory.
fn scratch_dir() -> Result<PathBuf, PublishError> {
    let dir = std::env::temp_dir().join(format!(
        "fastforge-aur-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

impl AppPublisher for AurPublisher {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        PUBLISHER_NAME
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
        let file = file_name(&artifact.display().to_string())?;
        if !file.contains(".pkg.tar.") {
            return Err(PublishError::General(format!(
                "aur publishes the pacman package (`.pkg.tar.zst`), not `{file}`"
            )));
        }
        let source_url = argument(config, &["source-url"])
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| {
                PublishError::General(
                    "`source-url` is required: the public URL the PKGBUILD downloads this package from, such as its GitHub release asset.".into(),
                )
            })?;

        let pkginfo = run(
            Command::new("bsdtar")
                .arg("-xOf")
                .arg(&artifact)
                .arg(".PKGINFO"),
            "bsdtar (reading .PKGINFO)",
        )?;
        let info = PkgInfo::parse(&pkginfo);
        if info.pkgname.is_empty() || info.pkgver.is_empty() {
            return Err(PublishError::General(format!(
                "{} has no pkgname or pkgver in .PKGINFO",
                artifact.display()
            )));
        }
        let install_script = Command::new("bsdtar")
            .arg("-xOf")
            .arg(&artifact)
            .arg(".INSTALL")
            .output()
            .ok()
            .filter(|out| out.status.success() && !out.stdout.is_empty())
            .map(|out| out.stdout);

        let name = argument(config, &["package", "package-name"])
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("{}-bin", info.pkgname));
        let mut hasher = Sha256::new();
        std::io::copy(&mut std::fs::File::open(&artifact)?, &mut hasher)?;
        let (pkgver, pkgrel) = info.version();
        let aur = AurPackage {
            maintainer: argument(config, &["maintainer"]).filter(|v| !v.trim().is_empty()),
            source_url,
            source_file: format!(
                "{}-{pkgver}-{pkgrel}-{}.pkg.tar.zst",
                info.pkgname, info.arch
            ),
            sha256: hex::encode(hasher.finalize()),
            install: install_script.as_ref().map(|_| format!("{name}.install")),
            name,
        };

        let git_url = argument(config, &["git-url"])
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("ssh://aur@aur.archlinux.org/{}.git", aur.name));
        let work = scratch_dir()?;
        let checkout = work.join(&aur.name);
        let result: Result<String, PublishError> = (|| {
            run(
                Command::new("git")
                    .arg("clone")
                    .arg(&git_url)
                    .arg(&checkout),
                "git clone",
            )?;
            std::fs::write(checkout.join("PKGBUILD"), pkgbuild(&info, &aur))?;
            std::fs::write(checkout.join(".SRCINFO"), srcinfo(&info, &aur))?;
            let mut files = vec!["PKGBUILD".to_string(), ".SRCINFO".to_string()];
            if let (Some(script), Some(install)) = (&install_script, &aur.install) {
                std::fs::write(checkout.join(install), script)?;
                files.push(install.clone());
            }
            run(git(&checkout).arg("add").args(&files), "git add")?;
            let changed = !git(&checkout)
                .args(["diff", "--cached", "--quiet"])
                .status()?
                .success();
            if !changed {
                return Ok(format!("{} is already up to date", aur.name));
            }
            let message = argument(config, &["commit-message"])
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| format!("Update to {pkgver}-{pkgrel}"));
            run(
                git(&checkout)
                    .args([
                        "-c",
                        "user.name=fastforge",
                        "-c",
                        "user.email=fastforge@localhost",
                    ])
                    .args(["commit", "-m", &message]),
                "git commit",
            )?;
            if argument(config, &["push"]).is_some_and(|v| v == "false" || v == "0") {
                return Ok(checkout.display().to_string());
            }
            // The AUR only accepts the master branch.
            run(
                git(&checkout).args(["push", "origin", "HEAD:master"]),
                "git push",
            )?;
            Ok(format!("https://aur.archlinux.org/packages/{}", aur.name))
        })();
        if !argument(config, &["push"]).is_some_and(|v| v == "false" || v == "0") {
            let _ = std::fs::remove_dir_all(&work);
        }
        Ok(PublishResult {
            success: true,
            message: result?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PKGINFO: &str = "pkgname = hello-world\npkgbase = hello-world\nxdata = pkgtype=pkg\npkgver = 1.2.3-4\npkgdesc = It's a demo\nurl = https://example.com\nbuilddate = 1\npackager = Jane <j@example.com>\nsize = 10\narch = x86_64\nlicense = MIT\ndepend = gtk3\ndepend = libmpv.so\noptdepend = zenity: dialogs\n";

    fn aur() -> AurPackage {
        AurPackage {
            name: "hello-world-bin".into(),
            maintainer: Some("Jane <j at example dot com>".into()),
            source_url: "https://example.com/hello.pkg.tar.zst".into(),
            source_file: "hello-world-1.2.3-4-x86_64.pkg.tar.zst".into(),
            sha256: "abc".into(),
            install: None,
        }
    }

    #[test]
    fn parses_pkginfo() {
        let info = PkgInfo::parse(PKGINFO);
        assert_eq!(info.pkgname, "hello-world");
        assert_eq!(info.version(), ("1.2.3", "4"));
        assert_eq!(info.depends, ["gtk3", "libmpv.so"]);
        assert_eq!(info.optdepends, ["zenity: dialogs"]);
    }

    #[test]
    fn pkgbuild_installs_the_package_and_provides_its_name() {
        let pkgbuild = pkgbuild(&PkgInfo::parse(PKGINFO), &aur());
        assert!(pkgbuild.starts_with("# Maintainer: Jane <j at example dot com>\npkgname=hello-world-bin\npkgver=1.2.3\npkgrel=4\n"));
        assert!(pkgbuild.contains("pkgdesc='It'\\''s a demo'\n"));
        assert!(pkgbuild.contains("depends=('gtk3' 'libmpv.so')\n"));
        assert!(pkgbuild.contains("provides=('hello-world')\nconflicts=('hello-world')\n"));
        assert!(pkgbuild.contains(
            "source=('hello-world-1.2.3-4-x86_64.pkg.tar.zst::https://example.com/hello.pkg.tar.zst')\n"
        ));
        assert!(pkgbuild.contains("--exclude .PKGINFO"));

        let srcinfo = srcinfo(&PkgInfo::parse(PKGINFO), &aur());
        assert!(srcinfo.starts_with(
            "pkgbase = hello-world-bin\n\tpkgdesc = It's a demo\n\tpkgver = 1.2.3\n\tpkgrel = 4\n"
        ));
        assert!(srcinfo.ends_with("\npkgname = hello-world-bin\n"));
    }
}
