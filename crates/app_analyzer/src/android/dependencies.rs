use serde_json::{Value, json};

/// Maven coordinates recorded by the Android Gradle plugin in every app bundle.
///
/// `BUNDLE-METADATA/com.android.tools.build.libraries/dependencies.pb` holds an
/// `AppDependencies` protobuf: a repeated `Library` (field 1), each holding a
/// `MavenLibrary` (field 1) of `groupId`, `artifactId` and `version` (fields 1
/// to 3). Only that path is walked, so an unexpected payload yields nothing
/// rather than nonsense.
pub fn parse(bytes: &[u8]) -> Vec<Value> {
    let mut dependencies: Vec<Value> = fields(bytes)
        .into_iter()
        .filter(|(number, _)| *number == 1)
        .filter_map(|(_, library)| maven_library(library))
        .collect();
    dependencies.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));
    dependencies.dedup_by(|left, right| left["name"] == right["name"]);
    dependencies
}

fn maven_library(library: &[u8]) -> Option<Value> {
    let maven = fields(library)
        .into_iter()
        .find(|(number, _)| *number == 1)
        .map(|(_, bytes)| bytes)?;

    let mut group = None;
    let mut artifact = None;
    let mut version = None;
    for (number, bytes) in fields(maven) {
        let text = coordinate(bytes)?;
        match number {
            1 => group = Some(text),
            2 => artifact = Some(text),
            3 => version = Some(text),
            _ => {}
        }
    }

    Some(json!({
        "name": format!("{}:{}", group?, artifact?),
        "version": version?,
    }))
}

/// Maven coordinates are plain ASCII; anything else means the bytes were not a
/// coordinate string and the message does not match the expected schema.
fn coordinate(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    (!text.is_empty() && text.chars().all(|c| c.is_ascii_graphic() || c == ' '))
        .then(|| text.to_string())
}

/// Splits a protobuf message into its length-delimited fields, skipping the
/// other wire types.
fn fields(bytes: &[u8]) -> Vec<(u64, &[u8])> {
    let mut fields = Vec::new();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let Some((tag, next)) = varint(bytes, cursor) else {
            break;
        };
        cursor = next;
        let number = tag >> 3;
        match tag & 0x07 {
            // Length-delimited: strings, bytes and nested messages.
            2 => {
                let Some((length, next)) = varint(bytes, cursor) else {
                    break;
                };
                let end = next
                    .checked_add(length as usize)
                    .filter(|end| *end <= bytes.len());
                let Some(end) = end else {
                    break;
                };
                fields.push((number, &bytes[next..end]));
                cursor = end;
            }
            0 => {
                let Some((_, next)) = varint(bytes, cursor) else {
                    break;
                };
                cursor = next;
            }
            5 => cursor += 4,
            1 => cursor += 8,
            _ => break,
        }
    }

    fields
}

fn varint(bytes: &[u8], start: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    let mut cursor = start;

    loop {
        let byte = *bytes.get(cursor)?;
        cursor += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, cursor));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}
