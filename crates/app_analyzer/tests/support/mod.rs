//! Builders for the synthetic packages the analyzer tests run against.
//!
//! Every test binary compiles this module separately and uses the part it
//! needs, so some builders are unused in any given one.
#![allow(dead_code)]

use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;

/// Writes a zip archive from `(name, contents)` pairs.
pub fn write_zip(path: &Path, entries: &[(&str, Vec<u8>)]) {
    let file = std::fs::File::create(path).expect("create archive");
    let mut writer = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    for (name, contents) in entries {
        writer.start_file(*name, options).expect("start entry");
        writer.write_all(contents).expect("write entry");
    }
    writer.finish().expect("finish archive");
}

/// A thin little-endian Mach-O executable carrying the load commands the
/// analyzer reads: the build version, an rpath, and a link table.
pub fn thin_mach_o(cpu_type: u32, platform: u32, libraries: &[&str]) -> Vec<u8> {
    let mut commands = Vec::new();
    commands.push(build_version_command(
        platform,
        version(15, 0, 0),
        version(17, 2, 0),
        &[
            (1, version(1500, 3, 9)),
            (2, version(5, 9, 0)),
            (3, version(1053, 12, 0)),
        ],
    ));
    commands.push(rpath_command("@executable_path/Frameworks"));
    commands.extend(libraries.iter().map(|name| dylib_command(name)));

    let command_count = commands.len() as u32;
    let commands: Vec<u8> = commands.concat();

    let mut binary = Vec::new();
    binary.extend_from_slice(&0xfeed_facfu32.to_le_bytes()); // MH_MAGIC_64
    binary.extend_from_slice(&cpu_type.to_le_bytes());
    binary.extend_from_slice(&0u32.to_le_bytes()); // cpusubtype
    binary.extend_from_slice(&2u32.to_le_bytes()); // MH_EXECUTE
    binary.extend_from_slice(&command_count.to_le_bytes());
    binary.extend_from_slice(&(commands.len() as u32).to_le_bytes());
    binary.extend_from_slice(&0u32.to_le_bytes()); // flags
    binary.extend_from_slice(&0u32.to_le_bytes()); // reserved
    binary.extend_from_slice(&commands);
    binary
}

/// Mach-O packs versions as `xxxx.yy.zz` into one 32-bit word.
pub fn version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 16) | (minor << 8) | patch
}

fn dylib_command(name: &str) -> Vec<u8> {
    let mut command = Vec::new();
    let payload = padded(name);
    command.extend_from_slice(&0x0cu32.to_le_bytes()); // LC_LOAD_DYLIB
    command.extend_from_slice(&((24 + payload.len()) as u32).to_le_bytes());
    command.extend_from_slice(&24u32.to_le_bytes()); // name offset
    command.extend_from_slice(&0u32.to_le_bytes()); // timestamp
    command.extend_from_slice(&version(1, 0, 0).to_le_bytes());
    command.extend_from_slice(&version(1, 0, 0).to_le_bytes());
    command.extend_from_slice(&payload);
    command
}

fn rpath_command(path: &str) -> Vec<u8> {
    let mut command = Vec::new();
    let payload = padded(path);
    command.extend_from_slice(&0x8000_001cu32.to_le_bytes()); // LC_RPATH
    command.extend_from_slice(&((12 + payload.len()) as u32).to_le_bytes());
    command.extend_from_slice(&12u32.to_le_bytes()); // path offset
    command.extend_from_slice(&payload);
    command
}

fn build_version_command(platform: u32, min_os: u32, sdk: u32, tools: &[(u32, u32)]) -> Vec<u8> {
    let mut command = Vec::new();
    command.extend_from_slice(&0x32u32.to_le_bytes()); // LC_BUILD_VERSION
    command.extend_from_slice(&((24 + tools.len() * 8) as u32).to_le_bytes());
    command.extend_from_slice(&platform.to_le_bytes());
    command.extend_from_slice(&min_os.to_le_bytes());
    command.extend_from_slice(&sdk.to_le_bytes());
    command.extend_from_slice(&(tools.len() as u32).to_le_bytes());
    for (tool, version) in tools {
        command.extend_from_slice(&tool.to_le_bytes());
        command.extend_from_slice(&version.to_le_bytes());
    }
    command
}

/// Load-command strings are NUL-terminated and padded to 8-byte alignment.
fn padded(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    while !bytes.len().is_multiple_of(8) {
        bytes.push(0);
    }
    bytes
}
