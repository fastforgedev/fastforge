use crate::error::DmgMakerError;
use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

const CLEAN_DS_STORE: &[u8] = include_bytes!("../assets/DSStore-clean");

#[derive(Debug, Clone)]
pub struct DsStoreBuilder {
    entries: Vec<Entry>,
    window: Window,
    icon_size: u32,
    background_alias: Option<Vec<u8>>,
    background_color: Option<[f64; 3]>,
}

#[derive(Debug, Clone)]
struct Window {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone)]
struct Entry {
    filename: String,
    structure_id: [u8; 4],
    buffer: Vec<u8>,
}

impl DsStoreBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            window: Window {
                x: 100,
                y: 100,
                width: 640,
                height: 480,
            },
            icon_size: 80,
            background_alias: None,
            background_color: None,
        }
    }

    pub fn set_background_alias(&mut self, alias_data: Vec<u8>) {
        self.background_alias = Some(alias_data);
    }

    pub fn set_background_color(&mut self, red: f64, green: f64, blue: f64) {
        self.background_color = Some([red, green, blue]);
    }

    pub fn set_icon_size(&mut self, size: u32) {
        self.icon_size = size;
    }

    pub fn set_icon_pos(&mut self, name: &str, x: i32, y: i32) {
        self.entries.push(Entry::iloc(name, x, y));
    }

    pub fn set_window_pos(&mut self, x: i32, y: i32) {
        self.window.x = x;
        self.window.y = y;
    }

    pub fn set_window_size(&mut self, width: i32, height: i32) {
        self.window.width = width;
        self.window.height = height + 22;
    }

    pub fn vsrn(&mut self, value: u32) {
        if value <= 1 {
            self.entries.push(Entry::vsrn(value));
        }
    }

    pub fn write(self, output_path: &Path) -> Result<(), DmgMakerError> {
        let mut entries = self.entries;
        entries.push(Entry::bwsp(&self.window)?);
        entries.push(Entry::icvp(
            self.icon_size,
            self.background_alias,
            self.background_color,
        )?);

        entries.sort_by(Entry::sort_order);

        let mut modified = vec![0_u8; 3840];
        write_u32_be(&mut modified, 0, 0);
        write_u32_be(&mut modified, 4, entries.len() as u32);

        let mut cursor = 8_usize;
        for entry in entries {
            if cursor + entry.buffer.len() > modified.len() {
                return Err(DmgMakerError::General(
                    "Too many .DS_Store entries for template block".to_string(),
                ));
            }
            modified[cursor..cursor + entry.buffer.len()].copy_from_slice(&entry.buffer);
            cursor += entry.buffer.len();
        }

        let mut out = CLEAN_DS_STORE.to_vec();
        write_u32_be(&mut out, 76, read_u32_be(&modified, 4));

        let ds_offset = 4100_usize;
        let end = ds_offset + modified.len();
        if out.len() < end {
            return Err(DmgMakerError::General(
                "Invalid DSStore-clean template".to_string(),
            ));
        }
        out[ds_offset..end].copy_from_slice(&modified);

        fs::write(output_path, out)?;
        Ok(())
    }
}

fn write_u32_be(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn read_u32_be(buf: &[u8], offset: usize) -> u32 {
    let mut bytes = [0_u8; 4];
    bytes.copy_from_slice(&buf[offset..offset + 4]);
    u32::from_be_bytes(bytes)
}

impl Entry {
    fn with_blob(filename: &str, structure_id: [u8; 4], data_type: [u8; 4], blob: Vec<u8>) -> Self {
        let filename_bytes = utf16be(filename);
        let filename_length = (filename_bytes.len() / 2) as u32;

        let mut buffer = Vec::with_capacity(4 + filename_bytes.len() + 4 + 4 + blob.len());
        buffer.extend_from_slice(&filename_length.to_be_bytes());
        buffer.extend_from_slice(&filename_bytes);
        buffer.extend_from_slice(&structure_id);
        buffer.extend_from_slice(&data_type);
        buffer.extend_from_slice(&blob);

        Self {
            filename: filename.to_string(),
            structure_id,
            buffer,
        }
    }

    fn iloc(filename: &str, x: i32, y: i32) -> Self {
        let mut blob = Vec::with_capacity(20);
        blob.extend_from_slice(&(16_u32).to_be_bytes());
        blob.extend_from_slice(&(x as u32).to_be_bytes());
        blob.extend_from_slice(&(y as u32).to_be_bytes());
        blob.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x00]);
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        Entry::with_blob(filename, *b"Iloc", *b"blob", blob)
    }

    fn vsrn(value: u32) -> Self {
        Entry::with_blob(".", *b"vSrn", *b"long", value.to_be_bytes().to_vec())
    }

    fn bwsp(window: &Window) -> Result<Self, DmgMakerError> {
        let window_bounds = format!(
            "{{{{{}, {}}}, {{{}, {}}}}}",
            window.x, window.y, window.width, window.height
        );

        let xml = format!(
            r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
  <key>ContainerShowSidebar</key><true/>
  <key>ShowPathbar</key><false/>
  <key>ShowSidebar</key><true/>
  <key>ShowStatusBar</key><false/>
  <key>ShowTabView</key><false/>
  <key>ShowToolbar</key><false/>
  <key>SidebarWidth</key><integer>0</integer>
  <key>WindowBounds</key><string>{}</string>
</dict>
</plist>
"#,
            escape_xml(&window_bounds)
        );

        let plist = plist_to_binary(xml.as_bytes())?;
        Ok(Entry::with_blob(".", *b"bwsp", *b"blob", wrap_blob(plist)))
    }

    fn icvp(
        icon_size: u32,
        background_alias: Option<Vec<u8>>,
        background_color: Option<[f64; 3]>,
    ) -> Result<Self, DmgMakerError> {
        let [red, green, blue] = background_color.unwrap_or([1.0, 1.0, 1.0]);
        let mut extra = String::new();

        if let Some(alias) = background_alias {
            extra.push_str("  <key>backgroundType</key><integer>2</integer>\n");
            extra.push_str(&format!(
                "  <key>backgroundImageAlias</key><data>{}</data>\n",
                base64_encode(&alias)
            ));
        } else {
            extra.push_str("  <key>backgroundType</key><integer>1</integer>\n");
        }

        let xml = format!(
            r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
{extra}  <key>backgroundColorRed</key><real>{red}</real>
  <key>backgroundColorGreen</key><real>{green}</real>
  <key>backgroundColorBlue</key><real>{blue}</real>
  <key>showIconPreview</key><true/>
  <key>showItemInfo</key><false/>
  <key>textSize</key><real>12</real>
  <key>iconSize</key><real>{icon_size}</real>
  <key>viewOptionsVersion</key><integer>1</integer>
  <key>gridSpacing</key><real>100</real>
  <key>gridOffsetX</key><real>0</real>
  <key>gridOffsetY</key><real>0</real>
  <key>labelOnBottom</key><true/>
  <key>arrangeBy</key><string>none</string>
</dict>
</plist>
"#
        );

        let plist = plist_to_binary(xml.as_bytes())?;
        Ok(Entry::with_blob(".", *b"icvp", *b"blob", wrap_blob(plist)))
    }

    fn sort_order(a: &Self, b: &Self) -> Ordering {
        match a.filename.cmp(&b.filename) {
            Ordering::Equal => a.structure_id.cmp(&b.structure_id),
            other => other,
        }
    }
}

fn wrap_blob(mut payload: Vec<u8>) -> Vec<u8> {
    let mut blob = Vec::with_capacity(payload.len() + 4);
    blob.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    blob.append(&mut payload);
    blob
}

fn utf16be(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 2);
    for unit in s.encode_utf16() {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

fn plist_to_binary(xml: &[u8]) -> Result<Vec<u8>, DmgMakerError> {
    let mut child = Command::new("plutil")
        .args(["-convert", "binary1", "-o", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| DmgMakerError::General("Failed to access plutil stdin".to_string()))?;
        std::io::Write::write_all(stdin, xml)?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(DmgMakerError::CommandFailed {
            command: "plutil".to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(output.stdout)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        out.push(TABLE[(n & 0x3F) as usize] as char);
        i += 3;
    }

    match input.len() - i {
        1 => {
            let n = (input[i] as u32) << 16;
            out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
            out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
            out.push('=');
        }
        _ => {}
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_base64_and_escapes_xml() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(
            escape_xml(r#"<a & "b" 'c'>"#),
            "&lt;a &amp; &quot;b&quot; &apos;c&apos;&gt;"
        );
    }

    #[test]
    fn builds_and_sorts_non_plist_entries() {
        let mut entries = [
            Entry::iloc("Applications", 420, 140),
            Entry::vsrn(1),
            Entry::iloc("App.app", 120, 140),
        ];
        entries.sort_by(Entry::sort_order);

        assert_eq!(entries[0].filename, ".");
        assert_eq!(&entries[0].structure_id, b"vSrn");
        assert_eq!(entries[1].filename, "App.app");
        assert_eq!(&entries[1].structure_id, b"Iloc");
        assert_eq!(entries[2].filename, "Applications");
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct ParsedEntry {
        filename: String,
        structure_id: String,
        data_type: String,
        payload: Vec<u8>,
    }

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("ds_store_test_{name}_{nanos}"))
    }

    fn read_ds_store_entries(path: &Path) -> Vec<ParsedEntry> {
        let bytes = fs::read(path).expect("read .DS_Store");
        let ds_offset = 4100_usize;
        let modified_len = 3840_usize;
        assert!(
            bytes.len() >= ds_offset + modified_len,
            "DS_Store size too small"
        );
        let modified = &bytes[ds_offset..ds_offset + modified_len];

        assert_eq!(read_u32_be(modified, 0), 0);
        let count = read_u32_be(modified, 4) as usize;
        let mut cursor = 8_usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let name_len = read_u32_be(modified, cursor) as usize;
            cursor += 4;
            let name_bytes_len = name_len * 2;
            let name_bytes = &modified[cursor..cursor + name_bytes_len];
            cursor += name_bytes_len;
            let filename = utf16be_decode(name_bytes);

            let structure_id = std::str::from_utf8(&modified[cursor..cursor + 4])
                .expect("structure id utf8")
                .to_string();
            cursor += 4;
            let data_type = std::str::from_utf8(&modified[cursor..cursor + 4])
                .expect("data type utf8")
                .to_string();
            cursor += 4;

            let payload = match data_type.as_str() {
                "blob" => {
                    let blob_len = read_u32_be(modified, cursor) as usize;
                    cursor += 4;
                    let data = modified[cursor..cursor + blob_len].to_vec();
                    cursor += blob_len;
                    data
                }
                "long" => {
                    let data = modified[cursor..cursor + 4].to_vec();
                    cursor += 4;
                    data
                }
                other => panic!("unexpected data type: {other}"),
            };

            entries.push(ParsedEntry {
                filename,
                structure_id,
                data_type,
                payload,
            });
        }
        entries
    }

    fn utf16be_decode(bytes: &[u8]) -> String {
        assert_eq!(bytes.len() % 2, 0);
        let mut units = Vec::with_capacity(bytes.len() / 2);
        for i in (0..bytes.len()).step_by(2) {
            units.push(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
        }
        String::from_utf16(&units).expect("utf16 decode")
    }

    fn plist_binary_to_xml(binary: &[u8]) -> String {
        let mut child = Command::new("plutil")
            .args(["-convert", "xml1", "-o", "-", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn plutil");
        {
            let stdin = child.stdin.as_mut().expect("plutil stdin");
            std::io::Write::write_all(stdin, binary).expect("write plist binary");
        }
        let output = child.wait_with_output().expect("wait plutil");
        assert!(
            output.status.success(),
            "plutil failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        String::from_utf8(output.stdout).expect("plist xml utf8")
    }

    fn extract_real(xml: &str, key: &str) -> f64 {
        let needle = format!("<key>{key}</key>");
        let start = xml
            .find(&needle)
            .unwrap_or_else(|| panic!("missing key {key}"));
        let rest = &xml[start + needle.len()..];
        let real_start = rest
            .find("<real>")
            .unwrap_or_else(|| panic!("missing <real> for {key}"));
        let real_end = rest[real_start + 6..]
            .find("</real>")
            .unwrap_or_else(|| panic!("missing </real> for {key}"));
        let value = &rest[real_start + 6..real_start + 6 + real_end];
        value.trim().parse::<f64>().expect("parse real")
    }

    fn extract_integer(xml: &str, key: &str) -> i64 {
        let needle = format!("<key>{key}</key>");
        let start = xml
            .find(&needle)
            .unwrap_or_else(|| panic!("missing key {key}"));
        let rest = &xml[start + needle.len()..];
        let int_start = rest
            .find("<integer>")
            .unwrap_or_else(|| panic!("missing <integer> for {key}"));
        let int_end = rest[int_start + 9..]
            .find("</integer>")
            .unwrap_or_else(|| panic!("missing </integer> for {key}"));
        let value = &rest[int_start + 9..int_start + 9 + int_end];
        value.trim().parse::<i64>().expect("parse integer")
    }

    fn extract_data(xml: &str, key: &str) -> String {
        let needle = format!("<key>{key}</key>");
        let start = xml
            .find(&needle)
            .unwrap_or_else(|| panic!("missing key {key}"));
        let rest = &xml[start + needle.len()..];
        let data_start = rest
            .find("<data>")
            .unwrap_or_else(|| panic!("missing <data> for {key}"));
        let data_end = rest[data_start + 6..]
            .find("</data>")
            .unwrap_or_else(|| panic!("missing </data> for {key}"));
        rest[data_start + 6..data_start + 6 + data_end]
            .trim()
            .to_string()
    }

    fn entry_map(entries: &[ParsedEntry]) -> HashMap<(String, String), ParsedEntry> {
        let mut map = HashMap::new();
        for entry in entries {
            map.insert(
                (entry.filename.clone(), entry.structure_id.clone()),
                ParsedEntry {
                    filename: entry.filename.clone(),
                    structure_id: entry.structure_id.clone(),
                    data_type: entry.data_type.clone(),
                    payload: entry.payload.clone(),
                },
            );
        }
        map
    }

    #[test]
    fn ds_store_write_and_parse_default() {
        let path = tmp_path("default");
        let mut builder = DsStoreBuilder::new();
        builder.set_icon_size(64);
        builder.set_window_pos(200, 150);
        builder.set_window_size(700, 500);
        builder.set_background_color(0.25, 0.5, 0.75);
        builder.set_icon_pos("App.app", 120, 140);
        builder.set_icon_pos("Applications", 420, 140);
        builder.vsrn(1);

        builder.write(&path).expect("write ds_store");

        let entries = read_ds_store_entries(&path);
        let map = entry_map(&entries);

        let iloc_app = map
            .get(&(String::from("App.app"), String::from("Iloc")))
            .expect("iloc App.app");
        assert_eq!(iloc_app.data_type, "blob");
        assert_eq!(iloc_app.payload.len(), 16);
        assert_eq!(read_u32_be(&iloc_app.payload, 0), 120_u32);
        assert_eq!(read_u32_be(&iloc_app.payload, 4), 140_u32);

        let iloc_apps = map
            .get(&(String::from("Applications"), String::from("Iloc")))
            .expect("iloc Applications");
        assert_eq!(iloc_apps.payload.len(), 16);
        assert_eq!(read_u32_be(&iloc_apps.payload, 0), 420_u32);
        assert_eq!(read_u32_be(&iloc_apps.payload, 4), 140_u32);

        let bwsp = map
            .get(&(String::from("."), String::from("bwsp")))
            .expect("bwsp");
        let bwsp_xml = plist_binary_to_xml(&bwsp.payload);
        assert!(bwsp_xml.contains("<key>WindowBounds</key>"));
        assert!(
            bwsp_xml.contains("{{200, 150}, {700, 522}}"),
            "window bounds missing or wrong"
        );

        let icvp = map
            .get(&(String::from("."), String::from("icvp")))
            .expect("icvp");
        let icvp_xml = plist_binary_to_xml(&icvp.payload);
        assert_eq!(extract_real(&icvp_xml, "iconSize"), 64.0);
        assert_eq!(extract_real(&icvp_xml, "backgroundColorRed"), 0.25);
        assert_eq!(extract_real(&icvp_xml, "backgroundColorGreen"), 0.5);
        assert_eq!(extract_real(&icvp_xml, "backgroundColorBlue"), 0.75);

        let vsrn = map
            .get(&(String::from("."), String::from("vSrn")))
            .expect("vSrn");
        assert_eq!(vsrn.data_type, "long");
        assert_eq!(read_u32_be(&vsrn.payload, 0), 1);
    }

    #[test]
    fn ds_store_background_alias_roundtrip() {
        let path = tmp_path("alias");
        let mut builder = DsStoreBuilder::new();
        let alias_data = vec![0_u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        builder.set_background_alias(alias_data.clone());
        builder.set_icon_pos("App.app", 100, 120);

        builder.write(&path).expect("write ds_store");

        let entries = read_ds_store_entries(&path);
        let map = entry_map(&entries);
        let icvp = map
            .get(&(String::from("."), String::from("icvp")))
            .expect("icvp");
        let icvp_xml = plist_binary_to_xml(&icvp.payload);
        assert_eq!(extract_integer(&icvp_xml, "backgroundType"), 2);
        let expected = base64_encode(&alias_data);
        let actual = extract_data(&icvp_xml, "backgroundImageAlias");
        assert_eq!(actual, expected);
    }
}
