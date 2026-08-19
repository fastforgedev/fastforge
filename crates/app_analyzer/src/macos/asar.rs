use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Largest directory listing we are willing to parse out of an archive.
const MAX_DIRECTORY_BYTES: u32 = 32 * 1024 * 1024;
/// Largest single file we extract; `package.json` is a few kilobytes.
const MAX_ENTRY_BYTES: u64 = 4 * 1024 * 1024;

/// Reads `package.json` from the root of an Electron `app.asar` archive — the
/// manifest that names the app's JavaScript dependencies.
///
/// An asar is a directory listing in JSON followed by the raw file contents:
/// four little-endian lengths, then the listing, then the data at offsets
/// counted from the end of the header.
pub fn read_package_json(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;

    let mut lengths = [0u8; 16];
    file.read_exact(&mut lengths).ok()?;
    let header_size = read_u32(&lengths, 4)?;
    let directory_size = read_u32(&lengths, 12)?;
    if directory_size == 0 || directory_size > MAX_DIRECTORY_BYTES {
        return None;
    }

    let mut directory = vec![0u8; directory_size as usize];
    file.read_exact(&mut directory).ok()?;
    let directory: Value = serde_json::from_slice(&directory).ok()?;

    let entry = directory.get("files")?.get("package.json")?;
    let size = entry.get("size")?.as_u64()?;
    if size == 0 || size > MAX_ENTRY_BYTES {
        return None;
    }
    // Offsets are stored as strings because they can exceed 2^53.
    let offset: u64 = entry.get("offset")?.as_str()?.parse().ok()?;
    let data_start = 8u64 + header_size as u64;

    file.seek(SeekFrom::Start(data_start.checked_add(offset)?))
        .ok()?;
    let mut contents = vec![0u8; size as usize];
    file.read_exact(&mut contents).ok()?;
    serde_json::from_slice(&contents).ok()
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(slice))
}
