//! The package root every Linux format is built from.
//!
//! [`stage`] lays out what gets installed, exactly where it is installed:
//! the bundle, the `/usr/bin` link, the desktop entry, the icons, the
//! metainfo and the `files/` overlays. Each packager then adds only its own
//! metadata (`DEBIAN/control`, the spec, `.PKGINFO`, `AppRun`), using the
//! helpers below to fill in what the format expects (installed size,
//! checksums, configuration files, the RPM file list).

use std::path::{Path, PathBuf};

use fastforge_core::{PackageConfig, PackageError};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use md5::{Digest, Md5};

use super::common::{DESKTOP_TEMPLATE, METAINFO_TEMPLATE, RawPackaging};
use crate::fs_util::copy_dir_contents;

/// The hicolor sizes a PNG icon is installed at (those not larger than the
/// source image).
const ICON_SIZES: [u32; 7] = [16, 32, 48, 64, 128, 256, 512];

/// Where things go inside the package root.
pub(crate) struct Layout {
    /// The bundle directory, relative to the root. Empty for an AppDir,
    /// whose root is the bundle.
    pub bundle_dir: PathBuf,
    /// Whether `usr/bin/<binary>` links to the binary in the bundle.
    pub bin_link: bool,
    /// The desktop entry's directory, relative to the root.
    pub desktop_dir: PathBuf,
    /// Whether the icon is also placed at the root (an AppDir needs it there).
    pub root_icon: bool,
    /// The top-level directories of the shared `files/` overlay that apply
    /// (`None`: all of them).
    pub shared_overlay: Option<&'static [&'static str]>,
}

impl Layout {
    /// A system package (DEB, RPM, Pacman): the bundle in `/opt/<binary>`
    /// and a `/usr/bin/<binary>` link to it.
    pub fn system(binary_name: &str) -> Self {
        Self {
            bundle_dir: Path::new("opt").join(binary_name),
            bin_link: true,
            desktop_dir: PathBuf::from("usr/share/applications"),
            root_icon: false,
            shared_overlay: None,
        }
    }

    /// An AppImage's AppDir: the bundle, desktop entry and icon at the root.
    /// Only `usr/` of the shared overlay applies: an AppImage installs
    /// nothing in `/etc` or `/opt`.
    pub fn app_dir() -> Self {
        Self {
            bundle_dir: PathBuf::new(),
            bin_link: false,
            desktop_dir: PathBuf::new(),
            root_icon: true,
            shared_overlay: Some(&["usr"]),
        }
    }

    /// The bundle's installed path (`/opt/<binary>`).
    pub fn install_dir(&self) -> String {
        format!("/{}", self.bundle_dir.display())
    }
}

/// What a packager contributes to the root besides the raw files.
pub(crate) struct Contents {
    /// The icon file (`make_config.yaml`'s `icon`, else `project.icon`).
    pub icon: Option<String>,
    /// A metainfo file configured in `make_config.yaml`.
    pub metainfo: Option<String>,
}

/// Builds the package root at `root`. `generated_desktop` provides the
/// desktop entry when there is no desktop template.
pub(crate) fn stage(
    raw: &RawPackaging,
    config: &PackageConfig,
    root: &Path,
    layout: &Layout,
    contents: Contents,
    generated_desktop: impl FnOnce() -> String,
) -> Result<(), PackageError> {
    let binary_name = &config.app_binary_name;
    copy_dir_contents(&config.build_output_dir, &root.join(&layout.bundle_dir))?;

    if layout.bin_link {
        let bin_dir = root.join("usr/bin");
        std::fs::create_dir_all(&bin_dir)?;
        let target = format!("{}/{}", layout.install_dir(), binary_name);
        symlink(&target, &bin_dir.join(binary_name))?;
    }

    if let Some(icon) = contents.icon {
        install_icon(Path::new(&icon), root, raw.app_id(), layout.root_icon)?;
    }

    // The metainfo template, else one configured in make_config.yaml.
    let metainfo = match raw.template(METAINFO_TEMPLATE)? {
        Some(path) => Some(path),
        None => match contents.metainfo {
            Some(path) if !Path::new(&path).is_file() => {
                return Err(PackageError::NotFound(format!(
                    "Metainfo {} path wasn't found",
                    path
                )));
            }
            other => other.map(PathBuf::from),
        },
    };
    if let Some(metainfo) = metainfo {
        let dir = root.join("usr/share/metainfo");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(
            dir.join(format!("{}.metainfo.xml", raw.app_id())),
            raw.render(&metainfo)?,
        )?;
    }

    let desktop_dir = root.join(&layout.desktop_dir);
    std::fs::create_dir_all(&desktop_dir)?;
    raw.write_or_generate(
        raw.template(DESKTOP_TEMPLATE)?,
        &desktop_dir.join(format!("{}.desktop", raw.app_id())),
        generated_desktop,
    )?;

    raw.install_overlay(root, layout.shared_overlay)
}

#[cfg(unix)]
fn symlink(target: &str, link: &Path) -> Result<(), PackageError> {
    if link.symlink_metadata().is_ok() {
        std::fs::remove_file(link)?;
    }
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink(_target: &str, _link: &Path) -> Result<(), PackageError> {
    Ok(())
}

/// Installs the icon as `<name>` in the hicolor theme: an SVG as scalable, a
/// PNG at every standard size up to its own, padded to a square first. With
/// `root_icon`, the 256 px (or largest) rendition, or the SVG, also goes to
/// the root.
pub(crate) fn install_icon(
    src: &Path,
    root: &Path,
    name: &str,
    root_icon: bool,
) -> Result<(), PackageError> {
    if !src.is_file() {
        return Err(PackageError::NotFound(format!(
            "provided icon {} path wasn't found",
            src.display()
        )));
    }
    let hicolor = root.join("usr/share/icons/hicolor");
    let ext = src
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "svg" => {
            let dir = hicolor.join("scalable/apps");
            std::fs::create_dir_all(&dir)?;
            std::fs::copy(src, dir.join(format!("{}.svg", name)))?;
            if root_icon {
                std::fs::copy(src, root.join(format!("{}.svg", name)))?;
            }
        }
        "png" => {
            let image = image::open(src).map_err(|e| {
                PackageError::General(format!("Failed to read icon {}: {}", src.display(), e))
            })?;
            let square = pad_to_square(image);
            let side = square.width();
            let mut sizes: Vec<u32> = ICON_SIZES.into_iter().filter(|s| *s <= side).collect();
            if sizes.is_empty() {
                sizes.push(side);
            }
            for size in &sizes {
                let dir = hicolor.join(format!("{size}x{size}/apps"));
                std::fs::create_dir_all(&dir)?;
                save_png(&square, *size, &dir.join(format!("{}.png", name)))?;
            }
            if root_icon {
                let size = sizes.iter().copied().filter(|s| *s <= 256).max();
                let size = size.unwrap_or(sizes[0]);
                save_png(&square, size, &root.join(format!("{}.png", name)))?;
            }
        }
        _ => {
            return Err(PackageError::General(format!(
                "Icon {} must be a PNG or an SVG file",
                src.display()
            )));
        }
    }
    Ok(())
}

/// Centers a non-square image on a transparent square canvas.
fn pad_to_square(image: DynamicImage) -> DynamicImage {
    let (width, height) = image.dimensions();
    if width == height {
        return image;
    }
    let side = width.max(height);
    let mut canvas = RgbaImage::new(side, side);
    image::imageops::overlay(
        &mut canvas,
        &image.to_rgba8(),
        i64::from((side - width) / 2),
        i64::from((side - height) / 2),
    );
    DynamicImage::ImageRgba8(canvas)
}

fn save_png(square: &DynamicImage, size: u32, path: &Path) -> Result<(), PackageError> {
    let resized = if square.width() == size {
        square.clone()
    } else {
        square.resize_exact(size, size, FilterType::Lanczos3)
    };
    resized
        .save_with_format(path, ImageFormat::Png)
        .map_err(|e| PackageError::General(format!("Failed to write {}: {}", path.display(), e)))
}

/// An entry of the package root, relative to it.
struct Entry {
    path: PathBuf,
    kind: Kind,
    len: u64,
}

#[derive(PartialEq)]
enum Kind {
    File,
    Dir,
    Symlink,
}

/// Every entry under `root` (sorted), skipping top-level entries in `skip`.
fn entries(root: &Path, skip: &[&str]) -> Result<Vec<Entry>, PackageError> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<Entry>) -> Result<(), PackageError> {
        let mut children: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let path = child.path();
            let meta = std::fs::symlink_metadata(&path)?;
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            if meta.file_type().is_symlink() {
                out.push(Entry {
                    path: rel,
                    kind: Kind::Symlink,
                    len: 0,
                });
            } else if meta.is_dir() {
                out.push(Entry {
                    path: rel,
                    kind: Kind::Dir,
                    len: 0,
                });
                walk(root, &path, out)?;
            } else {
                out.push(Entry {
                    path: rel,
                    kind: Kind::File,
                    len: meta.len(),
                });
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.retain(|entry| {
        let top = entry.path.components().next();
        !top.is_some_and(|c| skip.iter().any(|s| c.as_os_str() == *s))
    });
    Ok(out)
}

/// The installed size of `root` (minus `skip`): `(bytes, KiB)`, the KiB
/// counted like dpkg (each file rounded up, one per directory or link).
pub(crate) fn installed_size(root: &Path, skip: &[&str]) -> Result<(u64, u64), PackageError> {
    let mut bytes = 0;
    let mut kib = 0;
    for entry in entries(root, skip)? {
        match entry.kind {
            Kind::File => {
                bytes += entry.len;
                kib += entry.len.div_ceil(1024);
            }
            Kind::Dir | Kind::Symlink => kib += 1,
        }
    }
    Ok((bytes, kib))
}

/// `DEBIAN/md5sums`: `<md5>  <path>` for every regular file.
pub(crate) fn md5sums(root: &Path, skip: &[&str]) -> Result<String, PackageError> {
    let mut out = String::new();
    for entry in entries(root, skip)? {
        if entry.kind != Kind::File {
            continue;
        }
        let digest = Md5::digest(std::fs::read(root.join(&entry.path))?);
        let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        out.push_str(&format!("{}  {}\n", hex, slash_path(&entry.path)));
    }
    Ok(out)
}

/// The regular files under `etc/`, as installed paths (`/etc/...`): the
/// configuration files the package manager must preserve.
pub(crate) fn config_files(root: &Path) -> Result<Vec<String>, PackageError> {
    Ok(entries(root, &[])?
        .into_iter()
        .filter(|entry| entry.kind == Kind::File && entry.path.starts_with("etc"))
        .map(|entry| format!("/{}", slash_path(&entry.path)))
        .collect())
}

/// Directories that belong to the system (or to `hicolor-icon-theme`), so a
/// package must not own them.
const SYSTEM_DIRS: &[&str] = &[
    "etc",
    "opt",
    "usr",
    "usr/bin",
    "usr/lib",
    "usr/lib64",
    "usr/libexec",
    "usr/share",
    "usr/share/applications",
    "usr/share/doc",
    "usr/share/icons",
    "usr/share/licenses",
    "usr/share/metainfo",
    "usr/share/mime",
    "usr/share/mime/packages",
    "usr/share/pixmaps",
    "var",
    "var/lib",
];

fn is_system_dir(path: &Path) -> bool {
    let path = slash_path(path);
    SYSTEM_DIRS.contains(&path.as_str()) || path.starts_with("usr/share/icons/hicolor")
}

/// The `%files -f` list for an RPM: every file and link, `/etc` files as
/// `%config(noreplace)`, and every directory the package creates as `%dir`,
/// so removing the package removes them too. System directories, such as
/// `/usr/share/applications`, are left to the system.
pub(crate) fn rpm_file_list(root: &Path) -> Result<String, PackageError> {
    let mut out = String::new();
    for entry in entries(root, &[])? {
        let path = format!("/{}", slash_path(&entry.path));
        match entry.kind {
            Kind::Dir if is_system_dir(&entry.path) => {}
            Kind::Dir => out.push_str(&format!("%dir \"{}\"\n", path)),
            _ if entry.path.starts_with("etc") => {
                out.push_str(&format!("%config(noreplace) \"{}\"\n", path));
            }
            _ => out.push_str(&format!("\"{}\"\n", path)),
        }
    }
    Ok(out)
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The build time for package metadata: `SOURCE_DATE_EPOCH` when set (for
/// reproducible builds), else now.
pub(crate) fn build_date() -> u64 {
    std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, content: &[u8]) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn layouts() {
        let system = Layout::system("hola");
        assert_eq!(system.install_dir(), "/opt/hola");
        assert!(system.bin_link);
        let app_dir = Layout::app_dir();
        assert!(app_dir.root_icon && !app_dir.bin_link);
    }

    #[cfg(unix)]
    #[test]
    fn sizes_checksums_config_files_and_rpm_list() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "opt/hola/hola", &[0u8; 1500]);
        write(root, "opt/hola/lib/libapp.so", b"so");
        write(root, "etc/hola/app.conf", b"a=1\n");
        write(root, "DEBIAN/control", b"Package: hola\n");
        std::fs::create_dir_all(root.join("usr/bin")).unwrap();
        std::os::unix::fs::symlink("/opt/hola/hola", root.join("usr/bin/hola")).unwrap();

        let (bytes, kib) = installed_size(root, &["DEBIAN"]).unwrap();
        assert_eq!(bytes, 1500 + 2 + 4);
        // files: 2 + 1 + 1 KiB; dirs: opt, opt/hola, opt/hola/lib, etc,
        // etc/hola, usr, usr/bin; one link.
        assert_eq!(kib, 4 + 7 + 1);

        let sums = md5sums(root, &["DEBIAN"]).unwrap();
        assert!(sums.contains("  etc/hola/app.conf\n"));
        assert!(sums.contains("  opt/hola/lib/libapp.so\n"));
        assert!(!sums.contains("DEBIAN"));
        assert!(!sums.contains("usr/bin/hola"));
        assert_eq!(sums.lines().count(), 3);

        assert_eq!(config_files(root).unwrap(), vec!["/etc/hola/app.conf"]);

        std::fs::remove_dir_all(root.join("DEBIAN")).unwrap();
        let list = rpm_file_list(root).unwrap();
        assert_eq!(
            list,
            "%dir \"/etc/hola\"\n\
             %config(noreplace) \"/etc/hola/app.conf\"\n\
             %dir \"/opt/hola\"\n\
             \"/opt/hola/hola\"\n\
             %dir \"/opt/hola/lib\"\n\
             \"/opt/hola/lib/libapp.so\"\n\
             \"/usr/bin/hola\"\n"
        );
    }

    #[test]
    fn png_icons_are_squared_and_resized() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("logo.png");
        DynamicImage::ImageRgba8(RgbaImage::new(300, 200))
            .save_with_format(&src, ImageFormat::Png)
            .unwrap();
        let root = tmp.path().join("root");
        install_icon(&src, &root, "dev.example.hola", true).unwrap();
        let hicolor = root.join("usr/share/icons/hicolor");
        for size in [16, 32, 48, 64, 128, 256] {
            let icon =
                image::open(hicolor.join(format!("{size}x{size}/apps/dev.example.hola.png")))
                    .unwrap();
            assert_eq!(icon.dimensions(), (size, size));
        }
        assert!(!hicolor.join("512x512").exists());
        assert_eq!(
            image::open(root.join("dev.example.hola.png"))
                .unwrap()
                .dimensions(),
            (256, 256)
        );
    }

    #[test]
    fn svg_icons_are_scalable_and_others_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let svg = tmp.path().join("logo.svg");
        std::fs::write(&svg, "<svg/>").unwrap();
        let root = tmp.path().join("root");
        install_icon(&svg, &root, "hola", false).unwrap();
        assert!(
            root.join("usr/share/icons/hicolor/scalable/apps/hola.svg")
                .is_file()
        );
        assert!(!root.join("hola.svg").exists());

        let ico = tmp.path().join("logo.ico");
        std::fs::write(&ico, "x").unwrap();
        assert!(install_icon(&ico, &root, "hola", false).is_err());
    }
}
