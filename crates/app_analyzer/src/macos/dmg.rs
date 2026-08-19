use crate::checksum;
use crate::macos::signature::AssessmentType;
use crate::macos::{bundle, fs_stats, signature};
use crate::plist_util;
use crate::{command, json_util};
use fastforge_core::{AnalyzeConfig, AnalyzeError, AnalyzeResult, AppAnalyzer};
use plist::Dictionary;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A DMG only has to be attached long enough to read the bundle inside it, but
/// large images still take a while to mount.
const MOUNT_TIMEOUT: Duration = Duration::from_secs(120);
/// How deep to look for `.app` bundles inside the mounted volume.
const APP_SEARCH_DEPTH: usize = 4;

/// Distinguishes concurrent mounts of different images.
static MOUNT_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

pub struct MacOSDmgAnalyzer;

impl AppAnalyzer for MacOSDmgAnalyzer {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        "macos-dmg"
    }

    fn is_supported_on_current_platform(&self) -> bool {
        cfg!(target_os = "macos")
    }

    fn perform_analyze(&self, config: &AnalyzeConfig) -> Result<AnalyzeResult, AnalyzeError> {
        if !self.is_supported_on_current_platform() {
            return Err(AnalyzeError::General(
                "DMG analysis is only supported on macOS.".to_string(),
            ));
        }

        let dmg_path = Path::new(&config.path);
        if !dmg_path.is_file() {
            return Err(AnalyzeError::NotFound(format!(
                "Disk image not found: {}",
                config.path
            )));
        }

        let image_info = read_image_info(&config.path);
        if let Some(info) = image_info.as_ref()
            && property(info, "Encrypted").unwrap_or(false)
        {
            // Attaching would block on an interactive password prompt.
            return Err(AnalyzeError::General(
                "Encrypted disk images cannot be analyzed.".to_string(),
            ));
        }

        let mount_point = create_mount_point()?;
        let _mount_guard = mount_dmg(&config.path, &mount_point)?;

        let app_bundles = find_app_bundles(&mount_point, APP_SEARCH_DEPTH);
        let primary_app = app_bundles
            .first()
            .ok_or_else(|| AnalyzeError::NotFound("No .app bundle found in DMG".to_string()))?;

        let mut app = bundle::inspect(primary_app)?;
        // The mount point is a throwaway temp directory; report where the
        // bundle sits inside the volume instead.
        json_util::insert_text(
            &mut app,
            "path",
            relative_path(&mount_point, primary_app).or_else(|| {
                primary_app
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            }),
        );

        let mut data = Map::new();
        data.insert("platform".to_string(), Value::String("macos".to_string()));
        data.insert("format".to_string(), Value::String("dmg".to_string()));
        for key in ["identifier", "name", "version", "buildNumber"] {
            if let Some(value) = app.get(key) {
                data.insert(key.to_string(), value.clone());
            }
        }
        data.insert(
            "path".to_string(),
            Value::String(
                fs::canonicalize(dmg_path)
                    .unwrap_or_else(|_| dmg_path.to_path_buf())
                    .to_string_lossy()
                    .into_owned(),
            ),
        );
        json_util::insert_text(
            &mut data,
            "fileName",
            dmg_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
        );
        data.insert("sizeBytes".to_string(), json!(fs_stats::size_of(dmg_path)));
        json_util::insert_text(&mut data, "sha256", checksum::sha256_of(dmg_path));
        json_util::insert_value(
            &mut data,
            "codeSignature",
            signature::inspect(dmg_path, AssessmentType::Open),
        );
        json_util::insert_object(
            &mut data,
            "diskImage",
            image_info.map(disk_image_info).unwrap_or_default(),
        );
        json_util::insert_object(&mut data, "volume", volume_info(&mount_point));
        data.insert("app".to_string(), Value::Object(app));
        if app_bundles.len() > 1 {
            json_util::insert_array(
                &mut data,
                "apps",
                app_bundles
                    .iter()
                    .filter_map(|path| bundle::summary(path))
                    .collect(),
            );
        }

        log::info!("DMG analysis completed for {}", config.path);
        Ok(AnalyzeResult::new(true, Value::Object(data)))
    }
}

// ── Disk image ────────────────────────────────────────────────────────────────

fn read_image_info(dmg_path: &str) -> Option<Dictionary> {
    let output = command::run("hdiutil", &["imageinfo", "-plist", dmg_path])?;
    if !output.success {
        log::debug!("hdiutil imageinfo failed: {}", output.stderr_text().trim());
        return None;
    }
    plist_util::parse_dictionary(&output.stdout)
}

fn disk_image_info(info: Dictionary) -> Map<String, Value> {
    let mut image = Map::new();

    let format = plist_util::text(&info, "Format");
    json_util::insert_text(&mut image, "format", format.clone());
    json_util::insert_text(
        &mut image,
        "formatDescription",
        format
            .as_deref()
            .and_then(format_description)
            .map(str::to_string)
            // `hdiutil`'s own description is localized to the host language, so
            // it is only used when the format code is unknown.
            .or_else(|| plist_util::text(&info, "Format Description")),
    );
    json_util::insert_flag(&mut image, "compressed", property(&info, "Compressed"));
    json_util::insert_flag(&mut image, "encrypted", property(&info, "Encrypted"));
    json_util::insert_flag(&mut image, "checksummed", property(&info, "Checksummed"));
    json_util::insert_flag(
        &mut image,
        "softwareLicenseAgreement",
        property(&info, "Software License Agreement"),
    );
    json_util::insert_flag(
        &mut image,
        "kernelCompatible",
        property(&info, "Kernel Compatible"),
    );
    json_util::insert_text(
        &mut image,
        "checksumType",
        plist_util::text(&info, "Checksum Type"),
    );
    json_util::insert_text(
        &mut image,
        "checksumValue",
        plist_util::text(&info, "Checksum Value"),
    );

    if let Some(size) = info.get("Size Information").and_then(|v| v.as_dictionary()) {
        json_util::insert_value(&mut image, "totalBytes", integer(size, "Total Bytes"));
        json_util::insert_value(
            &mut image,
            "compressedBytes",
            integer(size, "Compressed Bytes"),
        );
        json_util::insert_value(
            &mut image,
            "compressionRatio",
            size.get("Compressed Ratio")
                .and_then(|value| value.as_real())
                .and_then(|ratio| serde_json::Number::from_f64(ratio).map(Value::Number)),
        );
        json_util::insert_value(&mut image, "sectorCount", integer(size, "Sector Count"));
    }

    if let Some(segments) = info.get("Segments").and_then(|value| value.as_array()) {
        image.insert("segmentCount".to_string(), json!(segments.len()));
    }

    if let Some(partitions) = info
        .get("partitions")
        .and_then(|value| value.as_dictionary())
    {
        json_util::insert_text(
            &mut image,
            "partitionScheme",
            plist_util::text(partitions, "partition-scheme"),
        );
        let block_size = partitions
            .get("block-size")
            .and_then(|value| value.as_unsigned_integer())
            .unwrap_or(512);
        json_util::insert_array(
            &mut image,
            "partitions",
            partition_list(partitions, block_size),
        );
    }

    image
}

/// Lists the real (non-synthesized) partitions of the image.
fn partition_list(partitions: &Dictionary, block_size: u64) -> Vec<Value> {
    let Some(entries) = partitions
        .get("partitions")
        .and_then(|value| value.as_array())
    else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|value| value.as_dictionary())
        .filter(|entry| !plist_util::flag(entry, "partition-synthesized").unwrap_or(false))
        .map(|entry| {
            let mut partition = Map::new();
            json_util::insert_text(
                &mut partition,
                "name",
                plist_util::text(entry, "partition-name"),
            );
            json_util::insert_text(
                &mut partition,
                "hint",
                plist_util::text(entry, "partition-hint"),
            );
            if let Some(length) = entry
                .get("partition-length")
                .and_then(|value| value.as_unsigned_integer())
            {
                partition.insert("sizeBytes".to_string(), json!(length * block_size));
            }
            Value::Object(partition)
        })
        .collect()
}

fn format_description(format: &str) -> Option<&'static str> {
    Some(match format {
        "UDZO" => "UDIF read-only, zlib-compressed",
        "UDBZ" => "UDIF read-only, bzip2-compressed",
        "ULFO" => "UDIF read-only, LZFSE-compressed",
        "ULMO" => "UDIF read-only, LZMA-compressed",
        "UDCO" => "UDIF read-only, ADC-compressed",
        "UDRO" => "UDIF read-only",
        "UDRW" => "UDIF read/write",
        "UDTO" => "DVD/CD master",
        "UDSP" => "UDIF sparse image",
        "UDSB" => "UDIF sparse bundle",
        "UFBI" => "UDIF entire image",
        "UDIF" => "UDIF",
        _ => return None,
    })
}

fn property(info: &Dictionary, key: &str) -> Option<bool> {
    let properties = info.get("Properties")?.as_dictionary()?;
    plist_util::flag(properties, key)
}

fn integer(dict: &Dictionary, key: &str) -> Option<Value> {
    dict.get(key)
        .and_then(|value| value.as_unsigned_integer())
        .map(Value::from)
}

// ── Mounted volume ────────────────────────────────────────────────────────────

/// Reports what the user actually sees after opening the DMG: the volume name,
/// its contents, and whether the drag-to-install layout is set up.
fn volume_info(mount_point: &Path) -> Map<String, Value> {
    let mut volume = Map::new();
    json_util::insert_text(&mut volume, "name", volume_name(mount_point));

    let stats = fs_stats::collect(mount_point, 0);
    volume.insert("sizeBytes".to_string(), json!(stats.size_bytes));
    volume.insert("fileCount".to_string(), json!(stats.file_count));

    let mut items = Vec::new();
    let mut has_applications_symlink = false;
    if let Ok(entries) = fs::read_dir(mount_point) {
        let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        paths.sort();
        for path in paths {
            let Some(name) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            let is_symlink = fs::symlink_metadata(&path)
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false);
            let target = is_symlink
                .then(|| fs::read_link(&path).ok())
                .flatten()
                .map(|target| target.to_string_lossy().into_owned());
            if target.as_deref() == Some("/Applications") {
                has_applications_symlink = true;
            }
            if name.starts_with('.') {
                continue;
            }

            let mut item = Map::new();
            item.insert("name".to_string(), Value::String(name.clone()));
            item.insert(
                "kind".to_string(),
                Value::String(item_kind(&path, is_symlink).to_string()),
            );
            if !is_symlink {
                item.insert("sizeBytes".to_string(), json!(fs_stats::size_of(&path)));
            }
            json_util::insert_text(&mut item, "target", target);
            items.push(Value::Object(item));
        }
    }

    volume.insert(
        "hasApplicationsSymlink".to_string(),
        Value::Bool(has_applications_symlink),
    );
    volume.insert(
        "hasCustomLayout".to_string(),
        Value::Bool(mount_point.join(".DS_Store").exists()),
    );
    volume.insert(
        "hasBackgroundImage".to_string(),
        Value::Bool(mount_point.join(".background").exists()),
    );
    volume.insert(
        "hasVolumeIcon".to_string(),
        Value::Bool(mount_point.join(".VolumeIcon.icns").exists()),
    );
    json_util::insert_array(&mut volume, "items", items);
    volume
}

fn item_kind(path: &Path, is_symlink: bool) -> &'static str {
    if is_symlink {
        "symlink"
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("app") {
        "app"
    } else if path.is_dir() {
        "directory"
    } else {
        "file"
    }
}

fn volume_name(mount_point: &Path) -> Option<String> {
    let output = command::run(
        "diskutil",
        &["info", "-plist", &mount_point.to_string_lossy()],
    )?;
    if !output.success {
        return None;
    }
    let info = plist_util::parse_dictionary(&output.stdout)?;
    plist_util::text(&info, "VolumeName")
}

/// Finds every `.app` bundle in the volume, shallowest first, so that the app
/// sitting at the root of the DMG is treated as the primary one.
fn find_app_bundles(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut current = vec![root.to_path_buf()];

    for _ in 0..max_depth {
        let mut next = Vec::new();
        for dir in current {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .map(|entry| entry.path())
                .filter(|path| {
                    !path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
                })
                .collect();
            paths.sort();
            for path in paths {
                if path.extension().and_then(|ext| ext.to_str()) == Some("app") {
                    found.push(path);
                } else {
                    next.push(path);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        current = next;
    }

    found
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    Some(path.strip_prefix(root).ok()?.to_string_lossy().into_owned())
}

// ── Mounting ──────────────────────────────────────────────────────────────────

fn create_mount_point() -> Result<PathBuf, AnalyzeError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            AnalyzeError::General(format!("Failed to create mount point timestamp: {}", e))
        })?
        .as_millis();
    // Two images analyzed in the same millisecond must not land on the same
    // mount point, so the timestamp is paired with a per-process counter.
    let sequence = MOUNT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mount_point = std::env::temp_dir().join(format!("fastforge-dmg-mount-{}-{}", ts, sequence));
    fs::create_dir_all(&mount_point).map_err(AnalyzeError::Io)?;
    Ok(mount_point)
}

fn mount_dmg(dmg_path: &str, mount_point: &Path) -> Result<MountedDmg, AnalyzeError> {
    let output = command::run_with_timeout(
        "hdiutil",
        &[
            "attach",
            "-nobrowse",
            "-readonly",
            "-noverify",
            "-noautoopen",
            "-mountpoint",
            mount_point.to_string_lossy().as_ref(),
            dmg_path,
        ],
        MOUNT_TIMEOUT,
    )
    .ok_or_else(|| {
        AnalyzeError::General(format!(
            "Failed to attach disk image within {:?}: {}",
            MOUNT_TIMEOUT, dmg_path
        ))
    })?;

    if !output.success {
        let _ = fs::remove_dir_all(mount_point);
        return Err(AnalyzeError::CommandFailed {
            command: "hdiutil".to_string(),
            stderr: output.stderr_text(),
        });
    }

    Ok(MountedDmg::new(mount_point.to_path_buf()))
}

struct MountedDmg {
    mount_point: PathBuf,
}

impl MountedDmg {
    fn new(mount_point: PathBuf) -> Self {
        Self { mount_point }
    }
}

impl Drop for MountedDmg {
    fn drop(&mut self) {
        let _ = Command::new("hdiutil")
            .args([
                "detach",
                self.mount_point.to_string_lossy().as_ref(),
                "-force",
            ])
            .output();
        let _ = fs::remove_dir_all(&self.mount_point);
    }
}
