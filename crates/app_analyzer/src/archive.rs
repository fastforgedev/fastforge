use crate::json_util;
use fastforge_core::AnalyzeError;
use serde_json::{Map, Value, json};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// How many of the biggest entries to report.
const LARGEST_ENTRY_LIMIT: usize = 10;
/// Upper bound for the metadata files read out of an archive.
const MAX_TEXT_ENTRY_BYTES: u64 = 4 * 1024 * 1024;

/// One file inside an APK or AAB.
pub struct Entry {
    pub name: String,
    pub size_bytes: u64,
    pub compressed_size_bytes: u64,
}

/// APKs and AABs are both zip archives, differing mostly in where things live:
/// an APK keeps them at the root, an AAB under a module directory such as
/// `base/`. Everything here therefore works against a path prefix.
pub struct Archive {
    archive: ZipArchive<File>,
    entries: Vec<Entry>,
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self, AnalyzeError> {
        let file = File::open(path).map_err(AnalyzeError::Io)?;
        let archive = ZipArchive::new(file)
            .map_err(|e| AnalyzeError::Parse(format!("Not a readable Android package: {}", e)))?;

        let entries = (0..archive.len())
            .filter_map(|index| {
                let entry = archive.name_for_index(index)?;
                Some(entry.to_string())
            })
            .collect::<Vec<String>>();
        let mut archive = archive;
        let entries = entries
            .into_iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let entry = archive.by_index_raw(index).ok()?;
                entry.is_file().then(|| Entry {
                    name,
                    size_bytes: entry.size(),
                    compressed_size_bytes: entry.compressed_size(),
                })
            })
            .collect();

        Ok(Self { archive, entries })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.iter().any(|entry| entry.name == name)
    }

    /// Whether any entry lives under `prefix` — used to test for directories,
    /// which zip archives do not have to record explicitly.
    pub fn contains_prefix(&self, prefix: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.name.starts_with(prefix))
    }

    pub fn names_with_prefix(&self, prefix: &str) -> Vec<&str> {
        self.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .filter(|name| name.starts_with(prefix))
            .collect()
    }

    /// Whether an entry matching `predicate` exists, for the many `lib/<abi>/…`
    /// style lookups where the ABI directory is not known up front.
    pub fn any(&self, predicate: impl Fn(&str) -> bool) -> bool {
        self.entries
            .iter()
            .any(|entry| predicate(entry.name.as_str()))
    }

    pub fn read_text(&mut self, name: &str) -> Option<String> {
        let bytes = self.read_bytes(name)?;
        String::from_utf8(bytes).ok()
    }

    pub fn read_bytes(&mut self, name: &str) -> Option<Vec<u8>> {
        self.read_bytes_capped(name, MAX_TEXT_ENTRY_BYTES)
    }

    /// Reads at most `limit` bytes of an entry, for the cases where only the
    /// start of a large file is needed.
    pub fn read_bytes_capped(&mut self, name: &str, limit: u64) -> Option<Vec<u8>> {
        let mut entry = self.archive.by_name(name).ok()?;
        let mut bytes = Vec::with_capacity(entry.size().min(limit) as usize);
        entry.by_ref().take(limit).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    /// Total size of the payload under `prefix`, uncompressed.
    pub fn size_under(&self, prefix: &str) -> u64 {
        self.entries
            .iter()
            .filter(|entry| entry.name.starts_with(prefix))
            .map(|entry| entry.size_bytes)
            .sum()
    }

    pub fn count_under(&self, prefix: &str) -> usize {
        self.names_with_prefix(prefix).len()
    }
}

/// Summarizes what an APK, or one module of an AAB, is made of.
///
/// `sizeBreakdown` uses compressed sizes because those are what the download
/// actually costs; `largestEntries` reports both.
pub fn contents_summary(entries: &[Entry], prefix: &str) -> Map<String, Value> {
    let scoped: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.name.starts_with(prefix))
        .collect();
    if scoped.is_empty() {
        return Map::new();
    }

    let dex: Vec<&&Entry> = scoped
        .iter()
        .filter(|entry| entry.name.ends_with(".dex"))
        .collect();
    let native_libraries = scoped
        .iter()
        .filter(|entry| entry.name.ends_with(".so"))
        .count();

    let mut contents = Map::new();
    contents.insert("entryCount".to_string(), json!(scoped.len()));
    contents.insert(
        "uncompressedSizeBytes".to_string(),
        json!(scoped.iter().map(|entry| entry.size_bytes).sum::<u64>()),
    );
    if !dex.is_empty() {
        contents.insert("dexCount".to_string(), json!(dex.len()));
        contents.insert(
            "dexSizeBytes".to_string(),
            json!(dex.iter().map(|entry| entry.size_bytes).sum::<u64>()),
        );
    }
    if native_libraries > 0 {
        contents.insert("nativeLibraryCount".to_string(), json!(native_libraries));
    }
    json_util::insert_object(
        &mut contents,
        "sizeBreakdown",
        size_breakdown(&scoped, prefix),
    );
    json_util::insert_array(
        &mut contents,
        "largestEntries",
        largest_entries(&scoped, prefix),
    );
    contents
}

/// Compressed size per top-level directory (or file) inside the archive.
fn size_breakdown(entries: &[&Entry], prefix: &str) -> Map<String, Value> {
    let mut totals: Vec<(String, u64)> = Vec::new();
    for entry in entries {
        let relative = entry.name.strip_prefix(prefix).unwrap_or(&entry.name);
        let group = relative
            .split_once('/')
            .map(|(head, _)| head)
            .unwrap_or(relative)
            .to_string();
        match totals.iter_mut().find(|(name, _)| *name == group) {
            Some((_, total)) => *total += entry.compressed_size_bytes,
            None => totals.push((group, entry.compressed_size_bytes)),
        }
    }

    totals.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    totals
        .into_iter()
        .map(|(name, size)| (name, json!(size)))
        .collect()
}

fn largest_entries(entries: &[&Entry], prefix: &str) -> Vec<Value> {
    let mut sorted: Vec<&&Entry> = entries.iter().collect();
    sorted.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));
    sorted
        .into_iter()
        .take(LARGEST_ENTRY_LIMIT)
        .map(|entry| {
            json!({
                "path": entry.name.strip_prefix(prefix).unwrap_or(&entry.name),
                "sizeBytes": entry.size_bytes,
                "compressedSizeBytes": entry.compressed_size_bytes,
            })
        })
        .collect()
}
