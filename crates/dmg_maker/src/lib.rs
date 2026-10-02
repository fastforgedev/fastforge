pub mod error;

mod config;
mod ds_store;

pub use crate::error::DmgMakerError;
use config::{AppDmgSpec, ContentType, LoadOptions};
use csscolorparser::Color;
use ds_store::DsStoreBuilder;
use regex::Regex;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CreateOptions {
    pub target: PathBuf,
    pub source: Option<PathBuf>,
    pub basepath: Option<PathBuf>,
    pub specification: Option<Value>,
}

pub fn create(options: CreateOptions) -> Result<(), DmgMakerError> {
    if cfg!(target_os = "macos") == false {
        return Err(DmgMakerError::UnsupportedPlatform(
            std::env::consts::OS.to_string(),
        ));
    }

    if options.target.exists() {
        return Err(DmgMakerError::TargetExists(options.target));
    }

    let _target_guard = create_empty_target(&options.target)?;

    let parsed = config::load_spec(&LoadOptions {
        source: options.source,
        basepath: options.basepath,
        specification: options.specification,
    })?;

    let spec = parsed.spec;
    let resolve_base = parsed.resolve_base;

    let mut state = BuildState::default();
    let result = build_dmg(&spec, &resolve_base, &options.target, &mut state);

    cleanup_state(&mut state);

    result?;

    Ok(())
}

#[derive(Default)]
struct BuildState {
    temporary_image_path: Option<PathBuf>,
    temporary_mount_path: Option<PathBuf>,
}

/// `create` rejects every non-macOS platform up front, so the DMG body below
/// is unreachable there. It still has to *compile*: `fastforge_app_packager`
/// depends on this crate unconditionally, so the Windows targets of the CLI
/// build it as well, and `std::os::unix` does not exist there. The Unix-only
/// pieces are therefore cfg-gated and fall back to this error.
#[cfg(not(unix))]
fn unsupported_platform() -> DmgMakerError {
    DmgMakerError::UnsupportedPlatform(std::env::consts::OS.to_string())
}

#[cfg(unix)]
fn create_symlink(target: impl AsRef<Path>, link: impl AsRef<Path>) -> Result<(), DmgMakerError> {
    Ok(std::os::unix::fs::symlink(target, link)?)
}

#[cfg(not(unix))]
fn create_symlink(_target: impl AsRef<Path>, _link: impl AsRef<Path>) -> Result<(), DmgMakerError> {
    Err(unsupported_platform())
}

fn build_dmg(
    spec: &AppDmgSpec,
    resolve_base: &Path,
    target: &Path,
    state: &mut BuildState,
) -> Result<(), DmgMakerError> {
    let files: Vec<_> = spec
        .contents
        .iter()
        .filter(|c| c.kind == ContentType::File)
        .collect();

    for file in &files {
        let abs = resolve(resolve_base, &file.path);
        if !abs.exists() {
            return Err(DmgMakerError::FileNotFound(format!(
                "\"{}\" not found at: {}",
                file.path,
                abs.display()
            )));
        }
    }

    let mut total_mb = 0.0_f64;
    for file in &files {
        let mb = du_sm(&resolve(resolve_base, &file.path))? as f64;
        total_mb += mb;
    }
    let image_mb = (total_mb * 1.5 + 32.0).ceil() as u64;

    let tmp_image = make_temp_image_path();
    run_cmd(
        "hdiutil",
        &[
            "create",
            tmp_image.to_string_lossy().as_ref(),
            "-ov",
            "-fs",
            spec.filesystem.as_deref().unwrap_or("HFS+"),
            "-size",
            &format!("{image_mb}m"),
            "-volname",
            &spec.title,
        ],
    )?;
    state.temporary_image_path = Some(tmp_image.clone());

    let attach_out = run_cmd_capture(
        "hdiutil",
        &[
            "attach",
            tmp_image.to_string_lossy().as_ref(),
            "-nobrowse",
            "-noverify",
            "-noautoopen",
        ],
    )?;

    let mount_path = parse_mount_path(&attach_out)?;
    state.temporary_mount_path = Some(mount_path.clone());

    let bkg_dir = mount_path.join(".background");
    fs::create_dir_all(&bkg_dir)?;

    let mut bkg_name: Option<PathBuf> = None;
    let mut bkg_size: Option<(i32, i32)> = None;

    if let Some(background) = &spec.background {
        let absolute = resolve(resolve_base, background);
        let retina = retina_path(&absolute);

        if retina.exists() {
            let output_name = format!(
                "{}.tiff",
                absolute
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| DmgMakerError::InvalidConfig(
                        "Invalid background file name".to_string()
                    ))?
            );
            let final_path = bkg_dir.join(&output_name);
            run_cmd(
                "tiffutil",
                &[
                    "-cathidpicheck",
                    absolute.to_string_lossy().as_ref(),
                    retina.to_string_lossy().as_ref(),
                    "-out",
                    final_path.to_string_lossy().as_ref(),
                ],
            )?;
            bkg_name = Some(PathBuf::from(".background").join(output_name));
        } else {
            let filename = absolute
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    DmgMakerError::InvalidConfig("Invalid background file name".to_string())
                })?;
            let final_path = bkg_dir.join(filename);
            fs::copy(&absolute, &final_path)?;
            bkg_name = Some(PathBuf::from(".background").join(filename));
        }

        bkg_size = Some(read_image_size(&absolute)?);
    }

    if let Some(icon) = &spec.icon {
        let final_path = mount_path.join(".VolumeIcon.icns");
        fs::copy(resolve(resolve_base, icon), &final_path)?;

        run_cmd(
            "xattr",
            &[
                "-wx",
                "com.apple.FinderInfo",
                "0000000000000000040000000000000000000000000000000000000000000000",
                mount_path.to_string_lossy().as_ref(),
            ],
        )?;
    }

    for entry in &spec.contents {
        match entry.kind {
            ContentType::Link => {
                let name = entry
                    .name
                    .clone()
                    .unwrap_or_else(|| basename(&entry.path).to_string());
                let final_path = mount_path.join(name);
                create_symlink(&entry.path, &final_path)?;
            }
            ContentType::File => {
                let src = resolve(resolve_base, &entry.path);
                let name = entry
                    .name
                    .clone()
                    .unwrap_or_else(|| basename(&entry.path).to_string());
                let dest = mount_path.join(name);
                run_cmd(
                    "cp",
                    &[
                        "-R",
                        src.to_string_lossy().as_ref(),
                        dest.to_string_lossy().as_ref(),
                    ],
                )?;
            }
            ContentType::Position => {}
        }
    }

    let mut ds = DsStoreBuilder::new();
    ds.vsrn(1);
    ds.set_icon_size(spec.icon_size.unwrap_or(80));

    if let Some(color) = &spec.background_color {
        let parsed = Color::from_html(color)
            .map_err(|e| DmgMakerError::General(format!("Invalid css color: {color}: {e}")))?
            .to_rgba8();
        ds.set_background_color(
            parsed[0] as f64 / 255.0,
            parsed[1] as f64 / 255.0,
            parsed[2] as f64 / 255.0,
        );
    }

    if let Some(ref bkg) = bkg_name {
        let absolute = mount_path.join(bkg);
        let alias = make_alias_record(&absolute)?;
        ds.set_background_alias(alias);
    }

    if let Some(size) = spec.window.as_ref().and_then(|w| w.size.as_ref()) {
        ds.set_window_size(size.width, size.height);
    } else if let Some((w, h)) = bkg_size {
        ds.set_window_size(w, h);
    } else {
        ds.set_window_size(640, 480);
    }

    if let Some(window) = &spec.window
        && let Some(position) = &window.position
    {
        ds.set_window_pos(position.x, position.y);
    }

    for entry in &spec.contents {
        let name = entry
            .name
            .as_deref()
            .unwrap_or_else(|| basename(&entry.path));
        ds.set_icon_pos(name, entry.x, entry.y);
    }

    ds.write(&mount_path.join(".DS_Store"))?;

    if spec.filesystem.as_deref() != Some("APFS") {
        let mut args = vec![
            "--folder".to_string(),
            mount_path.to_string_lossy().into_owned(),
        ];
        if std::env::consts::ARCH != "aarch64" {
            args.push("--openfolder".to_string());
            args.push(mount_path.to_string_lossy().into_owned());
        }
        run_cmd_owned("bless", &args)?;
    }

    detach_with_retry(&mount_path, 8)?;
    state.temporary_mount_path = None;

    run_cmd(
        "hdiutil",
        &[
            "convert",
            tmp_image.to_string_lossy().as_ref(),
            "-ov",
            "-format",
            spec.format.as_deref().unwrap_or("UDZO"),
            "-imagekey",
            "zlib-level=9",
            "-o",
            target.to_string_lossy().as_ref(),
        ],
    )?;

    if let Some(code_sign) = &spec.code_sign {
        let mut args = vec![
            "--verbose".to_string(),
            "--sign".to_string(),
            code_sign.signing_identity.clone(),
        ];
        if let Some(identifier) = &code_sign.identifier {
            args.push("--identifier".to_string());
            args.push(identifier.clone());
        }
        args.push(target.to_string_lossy().into_owned());
        run_cmd_owned("codesign", &args)?;
    }

    Ok(())
}

fn create_empty_target(path: &Path) -> Result<File, DmgMakerError> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| {
            DmgMakerError::General(format!("Failed to create target {}: {}", path.display(), e))
        })
}

fn cleanup_state(state: &mut BuildState) {
    if let Some(mount) = state.temporary_mount_path.take() {
        let _ = detach_with_retry(&mount, 8);
    }
    if let Some(image) = state.temporary_image_path.take() {
        let _ = fs::remove_file(image);
    }
}

fn resolve(base: &Path, path: &str) -> PathBuf {
    base.join(path)
}

fn basename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
}

fn du_sm(path: &Path) -> Result<u64, DmgMakerError> {
    let out = run_cmd_capture("du", &["-sm", path.to_string_lossy().as_ref()])?;
    let re = Regex::new(r"^([0-9]+)\t").expect("valid regex");
    let caps = re.captures(&out).ok_or_else(|| {
        DmgMakerError::General("du -sm: Unknown error parsing output".to_string())
    })?;
    let value = caps
        .get(1)
        .ok_or_else(|| DmgMakerError::General("du -sm: missing size".to_string()))?
        .as_str()
        .parse::<u64>()
        .map_err(|e| DmgMakerError::General(format!("du -sm: invalid size: {e}")))?;
    Ok(value)
}

fn read_image_size(path: &Path) -> Result<(i32, i32), DmgMakerError> {
    let out = run_cmd_capture(
        "sips",
        &[
            "-g",
            "pixelWidth",
            "-g",
            "pixelHeight",
            path.to_string_lossy().as_ref(),
        ],
    )?;

    let width_re = Regex::new(r"pixelWidth:\s+([0-9]+)").expect("valid regex");
    let height_re = Regex::new(r"pixelHeight:\s+([0-9]+)").expect("valid regex");

    let width = width_re
        .captures(&out)
        .and_then(|c| c.get(1))
        .ok_or_else(|| DmgMakerError::General("Failed to parse image width".to_string()))?
        .as_str()
        .parse::<i32>()
        .map_err(|e| DmgMakerError::General(format!("Invalid image width: {e}")))?;

    let height = height_re
        .captures(&out)
        .and_then(|c| c.get(1))
        .ok_or_else(|| DmgMakerError::General("Failed to parse image height".to_string()))?
        .as_str()
        .parse::<i32>()
        .map_err(|e| DmgMakerError::General(format!("Invalid image height: {e}")))?;

    Ok((width, height))
}

fn parse_mount_path(output: &str) -> Result<PathBuf, DmgMakerError> {
    let re = Regex::new(r"(?m)\s+(\/Volumes\/.+)$").expect("valid regex");
    let mount = re
        .captures(output)
        .and_then(|c| c.get(1))
        .ok_or_else(|| DmgMakerError::General("Failed to mount image".to_string()))?
        .as_str()
        .trim();
    Ok(PathBuf::from(mount))
}

fn retina_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("background");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("png");
    path.with_file_name(format!("{stem}@2x.{ext}"))
}

fn run_cmd(program: &str, args: &[&str]) -> Result<(), DmgMakerError> {
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        return Err(DmgMakerError::CommandFailed {
            command: format!("{} {}", program, args.join(" ")),
            stderr: String::new(),
        });
    }
    Ok(())
}

fn run_cmd_owned(program: &str, args: &[String]) -> Result<(), DmgMakerError> {
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        return Err(DmgMakerError::CommandFailed {
            command: format!("{} {}", program, args.join(" ")),
            stderr: String::new(),
        });
    }
    Ok(())
}

fn run_cmd_capture(program: &str, args: &[&str]) -> Result<String, DmgMakerError> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Err(DmgMakerError::CommandFailed {
            command: format!("{} {}", program, args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn detach_with_retry(path: &Path, max_attempts: usize) -> Result<(), DmgMakerError> {
    for attempt in 0..max_attempts {
        let output = Command::new("hdiutil")
            .args(["detach", path.to_string_lossy().as_ref()])
            .output()?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let is_busy = stderr.contains("Resource busy") || stderr.contains("code = 16");
        if !is_busy || attempt + 1 >= max_attempts {
            return Err(DmgMakerError::CommandFailed {
                command: format!("hdiutil detach {}", path.display()),
                stderr: stderr.trim().to_string(),
            });
        }

        let seconds = 1_u64 << attempt;
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }

    Err(DmgMakerError::General(format!(
        "Failed to detach {}",
        path.display()
    )))
}

fn make_temp_image_path() -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("dmg_maker-{now}.dmg"))
}

#[cfg(not(unix))]
fn make_alias_record(_path: &Path) -> Result<Vec<u8>, DmgMakerError> {
    Err(unsupported_platform())
}

#[cfg(unix)]
fn make_alias_record(path: &Path) -> Result<Vec<u8>, DmgMakerError> {
    let target_path = path.canonicalize()?;
    let target_meta = fs::metadata(&target_path)?;
    let parent_path = target_path.parent().ok_or_else(|| {
        DmgMakerError::General("Background path has no parent directory".to_string())
    })?;
    let parent_meta = fs::metadata(parent_path)?;
    let volume_path = find_volume_root(&target_path, &target_meta)?;
    let volume_meta = fs::metadata(&volume_path)?;

    let target_name = target_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| DmgMakerError::InvalidConfig("Invalid background file name".to_string()))?;
    let parent_name = parent_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("/");
    let volume_name = volume_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("/");

    let rel_to_volume = target_path
        .strip_prefix(&volume_path)
        .map_err(|_| DmgMakerError::General("Background not inside its volume root".to_string()))?
        .to_string_lossy();
    let rel_to_volume = format!("/{rel_to_volume}");
    let volume_path_str = volume_path.to_string_lossy();

    let target_pascal = utf16be_pascal(target_name)?;
    let volume_pascal = utf16be_pascal(volume_name)?;
    let extra: Vec<(i16, Vec<u8>)> = vec![
        (0, parent_name.as_bytes().to_vec()),
        (1, (parent_meta.ino() as u32).to_be_bytes().to_vec()),
        (14, target_pascal),
        (15, volume_pascal),
        (18, rel_to_volume.as_bytes().to_vec()),
        (19, volume_path_str.as_bytes().to_vec()),
    ];

    let base_len = 150_usize;
    let extra_len: usize = extra
        .iter()
        .map(|(_, data)| {
            let pad = data.len() % 2;
            4 + data.len() + pad
        })
        .sum();
    let total_len = base_len + extra_len + 4;

    let mut buf = vec![0_u8; total_len];
    buf[0..4].copy_from_slice(&0_u32.to_be_bytes());
    write_u16_be(&mut buf, 4, total_len as u16);
    write_u16_be(&mut buf, 6, 2);
    write_u16_be(&mut buf, 8, if target_meta.is_dir() { 1 } else { 0 });

    write_pascal_ascii(&mut buf, 10, 27, volume_name)?;
    write_u32_be(&mut buf, 38, apple_time_seconds(volume_meta.ctime())?);
    buf[42..44].copy_from_slice(b"H+");
    write_u16_be(
        &mut buf,
        44,
        if volume_path == Path::new("/") { 0 } else { 5 },
    );
    write_u32_be(&mut buf, 46, parent_meta.ino() as u32);
    write_pascal_ascii(&mut buf, 50, 63, target_name)?;
    write_u32_be(&mut buf, 114, target_meta.ino() as u32);
    write_u32_be(&mut buf, 118, apple_time_seconds(target_meta.ctime())?);
    write_i16_be(&mut buf, 130, -1);
    write_i16_be(&mut buf, 132, -1);
    write_u32_be(&mut buf, 134, 0x0000_0D02);
    write_u16_be(&mut buf, 138, 0);

    let mut pos = 150_usize;
    for (kind, data) in extra {
        write_i16_be(&mut buf, pos, kind);
        write_u16_be(&mut buf, pos + 2, data.len() as u16);
        buf[pos + 4..pos + 4 + data.len()].copy_from_slice(&data);
        pos += 4 + data.len();
        if data.len() % 2 == 1 {
            buf[pos] = 0;
            pos += 1;
        }
    }
    write_i16_be(&mut buf, pos, -1);
    write_u16_be(&mut buf, pos + 2, 0);

    Ok(buf)
}

#[cfg(unix)]
fn find_volume_root(
    start_path: &Path,
    start_meta: &fs::Metadata,
) -> Result<PathBuf, DmgMakerError> {
    let mut current = start_path.to_path_buf();
    let mut last_dev = start_meta.dev();
    let mut last_ino = start_meta.ino();
    loop {
        let parent = current.parent().unwrap_or(Path::new("/"));
        let parent_meta = fs::metadata(parent)?;
        if parent_meta.dev() != last_dev || parent_meta.ino() == last_ino {
            return Ok(current);
        }
        last_dev = parent_meta.dev();
        last_ino = parent_meta.ino();
        current = parent.to_path_buf();
    }
}

#[cfg(unix)]
fn utf16be_pascal(value: &str) -> Result<Vec<u8>, DmgMakerError> {
    let units: Vec<u16> = value.encode_utf16().collect();
    if units.len() > u16::MAX as usize {
        return Err(DmgMakerError::InvalidConfig(
            "String too long for alias field".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(2 + units.len() * 2);
    out.extend_from_slice(&(units.len() as u16).to_be_bytes());
    for u in units {
        out.extend_from_slice(&u.to_be_bytes());
    }
    Ok(out)
}

#[cfg(unix)]
fn write_pascal_ascii(
    buf: &mut [u8],
    offset: usize,
    max_len: usize,
    value: &str,
) -> Result<(), DmgMakerError> {
    let bytes = value.as_bytes();
    if bytes.len() > max_len {
        return Err(DmgMakerError::General(format!(
            "Alias field too long: {}",
            value
        )));
    }
    buf[offset] = bytes.len() as u8;
    for i in 0..max_len {
        buf[offset + 1 + i] = 0;
    }
    buf[offset + 1..offset + 1 + bytes.len()].copy_from_slice(bytes);
    Ok(())
}

#[cfg(unix)]
fn apple_time_seconds(unix_seconds: i64) -> Result<u32, DmgMakerError> {
    let value = unix_seconds.checked_add(2_082_844_800).ok_or_else(|| {
        DmgMakerError::General("Timestamp overflow while building alias".to_string())
    })?;
    if value < 0 || value > u32::MAX as i64 {
        return Err(DmgMakerError::General(
            "Timestamp out of range while building alias".to_string(),
        ));
    }
    Ok(value as u32)
}

#[cfg(unix)]
fn write_u16_be(buf: &mut [u8], offset: usize, value: u16) {
    buf[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

#[cfg(unix)]
fn write_i16_be(buf: &mut [u8], offset: usize, value: i16) {
    buf[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

#[cfg(unix)]
fn write_u32_be(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::parse_mount_path;

    #[test]
    fn parse_mount_path_multiline_output() {
        let output = "\
/dev/disk2              Apple_partition_scheme\n\
/dev/disk2s1            Apple_partition_map\n\
/dev/disk2s2            Apple_HFS                       /Volumes/Test Title\n";

        let mount = parse_mount_path(output).expect("should parse mount path");
        assert_eq!(mount.to_string_lossy(), "/Volumes/Test Title");
    }
}
