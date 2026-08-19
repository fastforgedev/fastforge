use crate::command;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const FAT_MAGIC: u32 = 0xcafe_babe;
const FAT_MAGIC_64: u32 = 0xcafe_babf;
const MH_MAGIC: u32 = 0xfeed_face;
const MH_CIGAM: u32 = 0xcefa_edfe;
const MH_MAGIC_64: u32 = 0xfeed_facf;
const MH_CIGAM_64: u32 = 0xcffa_edfe;

const CPU_ARCH_ABI64: u32 = 0x0100_0000;
const CPU_ARCH_ABI64_32: u32 = 0x0200_0000;
const CPU_TYPE_X86: u32 = 7;
const CPU_TYPE_ARM: u32 = 12;
const CPU_TYPE_POWERPC: u32 = 18;
const CPU_SUBTYPE_MASK: u32 = 0xff00_0000;

const LC_REQ_DYLD: u32 = 0x8000_0000;
const LC_LOAD_DYLIB: u32 = 0x0c;
const LC_LOAD_WEAK_DYLIB: u32 = 0x18 | LC_REQ_DYLD;
const LC_REEXPORT_DYLIB: u32 = 0x1f | LC_REQ_DYLD;
const LC_LOAD_UPWARD_DYLIB: u32 = 0x23 | LC_REQ_DYLD;
const LC_RPATH: u32 = 0x1c | LC_REQ_DYLD;
const LC_VERSION_MIN_MACOSX: u32 = 0x24;
const LC_BUILD_VERSION: u32 = 0x32;

/// Guards against a corrupt header claiming an absurd load-command table.
const MAX_LOAD_COMMANDS_BYTES: u32 = 16 * 1024 * 1024;
const MAX_FAT_SLICES: usize = 64;

/// A compiler or linker that contributed to the binary, from `LC_BUILD_VERSION`.
pub struct BuildTool {
    pub name: &'static str,
    pub version: Option<String>,
}

/// What a Mach-O executable reveals about how it was built and what it uses.
#[derive(Default)]
pub struct MachOInfo {
    pub architectures: Vec<String>,
    /// Install names of the linked libraries, e.g.
    /// `@rpath/FlutterMacOS.framework/Versions/A/FlutterMacOS`.
    pub libraries: Vec<String>,
    pub rpaths: Vec<String>,
    pub platform: Option<&'static str>,
    pub min_os: Option<String>,
    pub sdk: Option<String>,
    pub tools: Vec<BuildTool>,
}

/// Reads the header and load commands of a Mach-O executable.
///
/// For a universal binary every slice is listed under `architectures`, while
/// the link table is read from the slice matching the host architecture — the
/// slices of one binary are built from the same sources, so one is enough.
pub fn inspect(path: &Path) -> Option<MachOInfo> {
    let file = RefCell::new(File::open(path).ok()?);
    inspect_with(&|offset, length| {
        let mut file = file.borrow_mut();
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut buffer = vec![0u8; length];
        file.read_exact(&mut buffer).ok()?;
        Some(buffer)
    })
}

/// Same as [`inspect`], for a binary already held in memory — one read out of
/// an IPA, say. A buffer holding only the start of a universal binary still
/// yields its architectures, just not the link table of a later slice.
pub fn inspect_bytes(bytes: &[u8]) -> Option<MachOInfo> {
    inspect_with(&|offset, length| {
        let start = usize::try_from(offset).ok()?;
        bytes
            .get(start..start.checked_add(length)?)
            .map(<[u8]>::to_vec)
    })
}

/// Reader over the binary, so the same parsing serves a file and a buffer.
type ReadAt<'a> = dyn Fn(u64, usize) -> Option<Vec<u8>> + 'a;

fn inspect_with(read_at: &ReadAt) -> Option<MachOInfo> {
    let prefix = read_at(0, 8)?;
    let magic = u32::from_be_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]);

    match magic {
        FAT_MAGIC | FAT_MAGIC_64 => {
            let count = u32::from_be_bytes([prefix[4], prefix[5], prefix[6], prefix[7]]) as usize;
            if count == 0 || count > MAX_FAT_SLICES {
                return None;
            }
            let entry_size = if magic == FAT_MAGIC_64 { 32 } else { 20 };
            let table = read_at(8, count * entry_size)?;

            let mut architectures = Vec::with_capacity(count);
            let mut offsets = Vec::with_capacity(count);
            for index in 0..count {
                let entry = &table[index * entry_size..];
                let cpu_type = read_u32(entry, 0, false)?;
                let cpu_subtype = read_u32(entry, 4, false)?;
                let offset = if magic == FAT_MAGIC_64 {
                    read_u64_be(entry, 8)?
                } else {
                    read_u32(entry, 8, false)? as u64
                };
                architectures.push(arch_name(cpu_type, cpu_subtype));
                offsets.push(offset);
            }

            let slice = preferred_slice(&architectures);
            let mut info = read_slice(read_at, offsets[slice]).unwrap_or_default();
            info.architectures = architectures;
            Some(info)
        }
        MH_MAGIC | MH_MAGIC_64 | MH_CIGAM | MH_CIGAM_64 => read_slice(read_at, 0),
        _ => None,
    }
}

/// Architecture slices of a Mach-O executable, falling back to `lipo -archs`
/// when the header cannot be parsed.
pub fn architectures(path: &Path) -> Vec<String> {
    match inspect(path) {
        Some(info) if !info.architectures.is_empty() => info.architectures,
        _ => lipo_architectures(path),
    }
}

fn read_slice(read_at: &ReadAt, offset: u64) -> Option<MachOInfo> {
    let header = read_at(offset, 32)?;
    let magic = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    // The header is written in the byte order of its own architecture, so a
    // big-endian read returns the canonical magic only for a big-endian binary.
    let (little_endian, is_64) = match magic {
        MH_MAGIC => (false, false),
        MH_MAGIC_64 => (false, true),
        MH_CIGAM => (true, false),
        MH_CIGAM_64 => (true, true),
        _ => return None,
    };

    let cpu_type = read_u32(&header, 4, little_endian)?;
    let cpu_subtype = read_u32(&header, 8, little_endian)?;
    let command_count = read_u32(&header, 16, little_endian)?;
    let commands_size = read_u32(&header, 20, little_endian)?;
    if commands_size == 0 || commands_size > MAX_LOAD_COMMANDS_BYTES {
        return None;
    }

    let header_size = if is_64 { 32 } else { 28 };
    let commands = read_at(offset + header_size, commands_size as usize)?;

    let mut info = MachOInfo {
        architectures: vec![arch_name(cpu_type, cpu_subtype)],
        ..MachOInfo::default()
    };
    parse_load_commands(&commands, command_count, little_endian, &mut info);
    Some(info)
}

fn parse_load_commands(
    commands: &[u8],
    command_count: u32,
    little_endian: bool,
    info: &mut MachOInfo,
) {
    let mut cursor = 0usize;
    for _ in 0..command_count {
        let Some(command) = read_u32(commands, cursor, little_endian) else {
            return;
        };
        let Some(size) = read_u32(commands, cursor + 4, little_endian) else {
            return;
        };
        let size = size as usize;
        // Every load command is at least a `cmd`/`cmdsize` pair; anything
        // smaller (or overrunning the table) means the binary is malformed.
        if size < 8 || cursor + size > commands.len() {
            return;
        }
        let body = &commands[cursor..cursor + size];

        match command {
            LC_LOAD_DYLIB | LC_LOAD_WEAK_DYLIB | LC_REEXPORT_DYLIB | LC_LOAD_UPWARD_DYLIB => {
                if let Some(path) = lc_str(body, read_u32(body, 8, little_endian)) {
                    info.libraries.push(path);
                }
            }
            LC_RPATH => {
                if let Some(path) = lc_str(body, read_u32(body, 8, little_endian)) {
                    info.rpaths.push(path);
                }
            }
            LC_BUILD_VERSION => {
                info.platform = read_u32(body, 8, little_endian).and_then(platform_name);
                info.min_os = read_u32(body, 12, little_endian).and_then(format_version);
                info.sdk = read_u32(body, 16, little_endian).and_then(format_version);
                let tool_count = read_u32(body, 20, little_endian).unwrap_or(0) as usize;
                for index in 0..tool_count {
                    let tool_offset = 24 + index * 8;
                    if tool_offset + 8 > size {
                        break;
                    }
                    let Some(name) = read_u32(body, tool_offset, little_endian).and_then(tool_name)
                    else {
                        continue;
                    };
                    info.tools.push(BuildTool {
                        name,
                        version: read_u32(body, tool_offset + 4, little_endian)
                            .and_then(format_version),
                    });
                }
            }
            // Superseded by LC_BUILD_VERSION, but still emitted by older tools.
            LC_VERSION_MIN_MACOSX => {
                info.platform.get_or_insert("macOS");
                info.min_os = read_u32(body, 8, little_endian).and_then(format_version);
                info.sdk = read_u32(body, 12, little_endian).and_then(format_version);
            }
            _ => {}
        }

        cursor += size;
    }
}

/// Picks the slice to read the link table from, preferring the host
/// architecture so the reported dependencies match what actually runs here.
fn preferred_slice(architectures: &[String]) -> usize {
    let preferred: &[&str] = if cfg!(target_arch = "aarch64") {
        &["arm64", "arm64e"]
    } else {
        &["x86_64", "x86_64h"]
    };
    preferred
        .iter()
        .find_map(|wanted| architectures.iter().position(|arch| arch == wanted))
        .unwrap_or(0)
}

fn lipo_architectures(path: &Path) -> Vec<String> {
    let Some(output) = command::run("lipo", &["-archs", &path.to_string_lossy()]) else {
        return Vec::new();
    };
    if !output.success {
        return Vec::new();
    }
    output
        .stdout_text()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn arch_name(cpu_type: u32, cpu_subtype: u32) -> String {
    let subtype = cpu_subtype & !CPU_SUBTYPE_MASK;
    match cpu_type {
        t if t == CPU_TYPE_X86 | CPU_ARCH_ABI64 => match subtype {
            8 => "x86_64h".to_string(),
            _ => "x86_64".to_string(),
        },
        CPU_TYPE_X86 => "i386".to_string(),
        t if t == CPU_TYPE_ARM | CPU_ARCH_ABI64 => match subtype {
            2 => "arm64e".to_string(),
            _ => "arm64".to_string(),
        },
        t if t == CPU_TYPE_ARM | CPU_ARCH_ABI64_32 => "arm64_32".to_string(),
        CPU_TYPE_ARM => "arm".to_string(),
        t if t == CPU_TYPE_POWERPC | CPU_ARCH_ABI64 => "ppc64".to_string(),
        CPU_TYPE_POWERPC => "ppc".to_string(),
        other => format!("unknown(cputype {})", other),
    }
}

fn platform_name(platform: u32) -> Option<&'static str> {
    Some(match platform {
        1 => "macOS",
        2 => "iOS",
        3 => "tvOS",
        4 => "watchOS",
        5 => "bridgeOS",
        6 => "Mac Catalyst",
        7 => "iOS Simulator",
        8 => "tvOS Simulator",
        9 => "watchOS Simulator",
        10 => "DriverKit",
        11 => "visionOS",
        12 => "visionOS Simulator",
        _ => return None,
    })
}

fn tool_name(tool: u32) -> Option<&'static str> {
    Some(match tool {
        1 => "clang",
        2 => "swift",
        3 => "ld",
        4 => "lld",
        _ => return None,
    })
}

/// Mach-O packs versions as `xxxx.yy.zz` in a single 32-bit word.
fn format_version(version: u32) -> Option<String> {
    if version == 0 {
        return None;
    }
    let major = version >> 16;
    let minor = (version >> 8) & 0xff;
    let patch = version & 0xff;
    Some(if patch == 0 {
        format!("{}.{}", major, minor)
    } else {
        format!("{}.{}.{}", major, minor, patch)
    })
}

/// Reads a NUL-terminated `lc_str`, whose offset is relative to the start of
/// the load command that carries it.
fn lc_str(body: &[u8], offset: Option<u32>) -> Option<String> {
    let offset = offset? as usize;
    let bytes = body.get(offset..)?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn read_u32(bytes: &[u8], offset: usize, little_endian: bool) -> Option<u32> {
    let slice: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(if little_endian {
        u32::from_le_bytes(slice)
    } else {
        u32::from_be_bytes(slice)
    })
}

fn read_u64_be(bytes: &[u8], offset: usize) -> Option<u64> {
    let slice: [u8; 8] = bytes.get(offset..offset + 8)?.try_into().ok()?;
    Some(u64::from_be_bytes(slice))
}
