//! A project's `Package.appxmanifest` (WinUI 3 / Windows App SDK projects),
//! read for its identity and adapted into the `AppxManifest.xml` that
//! `makeappx` packs. The document is edited textually, so every element the
//! project declares (capabilities, extensions, dependencies) is preserved.

use std::path::Path;

use fastforge_core::PackageError;

/// Escapes a value for use in XML text or attribute content.
pub fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Identity values written into the manifest's `<Identity>` element and the
/// executable substituted for MSBuild's `$targetnametoken$`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestIdentity<'a> {
    pub name: &'a str,
    pub publisher: &'a str,
    pub version: &'a str,
    pub architecture: &'a str,
    /// The executable file name in the package root, e.g. `MyApp.exe`.
    pub executable: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppxManifest {
    pub text: String,
    pub identity_name: Option<String>,
    pub publisher: Option<String>,
    /// The four-part `Identity Version`.
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub publisher_display_name: Option<String>,
}

impl AppxManifest {
    pub fn load(path: &Path) -> Result<Self, PackageError> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            PackageError::General(format!("Failed to read {}: {}", path.display(), e))
        })?;
        Self::parse(&text).map_err(|e| PackageError::General(format!("{}: {}", path.display(), e)))
    }

    pub fn parse(text: &str) -> Result<Self, PackageError> {
        let (start, end) = find_start_tag(text, "Identity").ok_or_else(|| {
            PackageError::General("the manifest has no <Identity> element".into())
        })?;
        let identity = attributes(&text[start..end]);
        let attribute = |name: &str| {
            identity
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
                .filter(|value| !value.is_empty())
        };
        Ok(Self {
            text: text.to_string(),
            identity_name: attribute("Name"),
            publisher: attribute("Publisher"),
            version: attribute("Version"),
            display_name: element_text(text, "DisplayName"),
            publisher_display_name: element_text(text, "PublisherDisplayName"),
        })
    }

    /// The app version derived from the identity version: `a.b.c.d` becomes
    /// `a.b.c+d`, or `a.b.c` when `d` is `0`.
    pub fn app_version(&self) -> Option<String> {
        let version = self.version.as_deref()?;
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 4
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Some(version.to_string());
        }
        let build_name = parts[..3].join(".");
        Some(if parts[3].trim_start_matches('0').is_empty() {
            build_name
        } else {
            format!("{build_name}+{}", parts[3])
        })
    }

    /// The manifest with the resolved identity, MSBuild's `$targetnametoken$`
    /// and `$targetentrypoint$` placeholders replaced, and the `x-generate`
    /// resource language (which only MSBuild expands) replaced by `en-us`.
    pub fn render(&self, identity: &ManifestIdentity<'_>) -> String {
        let stem = identity
            .executable
            .strip_suffix(".exe")
            .or_else(|| identity.executable.strip_suffix(".EXE"))
            .unwrap_or(identity.executable);
        let mut text = self
            .text
            .replace("$targetnametoken$", &xml_escape(stem))
            .replace("$targetentrypoint$", "Windows.FullTrustApplication")
            .replace("Language=\"x-generate\"", "Language=\"en-us\"")
            .replace("Language='x-generate'", "Language='en-us'");
        if let Some((start, end)) = find_start_tag(&text, "Identity") {
            let mut tag = text[start..end].to_string();
            for (name, value) in [
                ("Name", identity.name),
                ("Publisher", identity.publisher),
                ("Version", identity.version),
                ("ProcessorArchitecture", identity.architecture),
            ] {
                tag = set_attribute(&tag, name, value);
            }
            text.replace_range(start..end, &tag);
        }
        text
    }
}

/// Byte ranges of XML comments, which element searches skip.
fn comment_ranges(xml: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut from = 0;
    while let Some(start) = xml[from..].find("<!--") {
        let start = from + start;
        let end = xml[start..]
            .find("-->")
            .map_or(xml.len(), |e| start + e + 3);
        ranges.push((start, end));
        from = end;
    }
    ranges
}

/// The range of the first `<name ...>` or `<name .../>` start tag outside
/// comments, including its closing `>`.
fn find_start_tag(xml: &str, name: &str) -> Option<(usize, usize)> {
    let comments = comment_ranges(xml);
    let pattern = format!("<{name}");
    let mut from = 0;
    while let Some(found) = xml[from..].find(&pattern) {
        let start = from + found;
        from = start + pattern.len();
        if comments.iter().any(|(s, e)| start >= *s && start < *e) {
            continue;
        }
        let next = xml[start + pattern.len()..].chars().next();
        if !matches!(next, Some(c) if c.is_whitespace() || c == '/' || c == '>') {
            continue;
        }
        let end = tag_end(xml, start)?;
        return Some((start, end));
    }
    None
}

/// The index just past the `>` closing the tag that starts at `start`,
/// ignoring `>` inside quoted attribute values.
fn tag_end(xml: &str, start: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (offset, c) in xml[start..].char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"') | (None, '\'') => quote = Some(c),
            (None, '>') => return Some(start + offset + 1),
            _ => {}
        }
    }
    None
}

/// `(name, unescaped value)` pairs of a start tag.
fn attributes(tag: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let body = tag.trim_start_matches('<');
    let body = body.trim_end_matches('>').trim_end_matches('/');
    let mut rest = match body.find(char::is_whitespace) {
        Some(i) => &body[i..],
        None => return pairs,
    };
    loop {
        rest = rest.trim_start();
        let Some(eq) = rest.find('=') else { break };
        let name = rest[..eq].trim().to_string();
        let after = rest[eq + 1..].trim_start();
        let Some(quote) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            break;
        };
        let value_start = 1;
        let Some(value_len) = after[value_start..].find(quote) else {
            break;
        };
        let value = &after[value_start..value_start + value_len];
        if !name.is_empty() {
            pairs.push((name, xml_unescape(value)));
        }
        rest = &after[value_start + value_len + 1..];
    }
    pairs
}

/// `tag` with attribute `name` set to `value` (replaced in place, or appended
/// before the closing `>` / `/>`).
fn set_attribute(tag: &str, name: &str, value: &str) -> String {
    let escaped = xml_escape(value);
    let mut rest = tag;
    let mut offset = 0;
    while let Some(found) = rest.find(name) {
        let at = offset + found;
        let before = tag[..at].chars().next_back();
        let after = &tag[at + name.len()..];
        let after_trimmed = after.trim_start();
        if before.is_some_and(char::is_whitespace)
            && let Some(after_eq) = after_trimmed.strip_prefix('=')
        {
            let after_eq_trimmed = after_eq.trim_start();
            if let Some(quote) = after_eq_trimmed
                .chars()
                .next()
                .filter(|c| *c == '"' || *c == '\'')
            {
                let value_start = tag.len() - after_eq_trimmed.len() + quote.len_utf8();
                if let Some(len) = tag[value_start..].find(quote) {
                    let mut out = tag[..value_start].to_string();
                    out.push_str(&escaped);
                    out.push_str(&tag[value_start + len..]);
                    return out;
                }
            }
        }
        offset = at + name.len();
        rest = &tag[offset..];
    }
    let trimmed = tag.trim_end_matches('>');
    let (head, tail) = match trimmed.strip_suffix('/') {
        Some(head) => (head.trim_end(), "/>"),
        None => (trimmed.trim_end(), ">"),
    };
    format!("{head} {name}=\"{escaped}\"{tail}")
}

/// The unescaped text of the first `<name>text</name>` element outside
/// comments, when it holds no child elements.
fn element_text(xml: &str, name: &str) -> Option<String> {
    let (_, end) = find_start_tag(xml, name)?;
    if xml[..end].ends_with("/>") {
        return None;
    }
    let close = format!("</{name}>");
    let text = &xml[end..end + xml[end..].find(&close)?];
    if text.contains('<') {
        return None;
    }
    let text = xml_unescape(text.trim());
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- <Identity Name="commented.out" Publisher="CN=Nobody" Version="9.9.9.9"/> -->
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">
  <Identity
    Name="DazzLabs.DazzCal"
    Publisher="CN=Dazz &amp; Labs"
    Version="1.2.3.4" />
  <Properties>
    <DisplayName>DazzCal</DisplayName>
    <PublisherDisplayName>Dazz Labs</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>
  <Resources>
    <Resource Language="x-generate"/>
  </Resources>
  <Applications>
    <Application Id="App" Executable="$targetnametoken$.exe" EntryPoint="$targetentrypoint$">
      <uap:VisualElements DisplayName="DazzCal" Description="Calendar" BackgroundColor="transparent" Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png">
        <uap:DefaultTile Wide310x150Logo="Assets\Wide310x150Logo.png" />
      </uap:VisualElements>
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
"#;

    #[test]
    fn parses_identity_and_properties_outside_comments() {
        let manifest = AppxManifest::parse(MANIFEST).unwrap();
        assert_eq!(manifest.identity_name.as_deref(), Some("DazzLabs.DazzCal"));
        assert_eq!(manifest.publisher.as_deref(), Some("CN=Dazz & Labs"));
        assert_eq!(manifest.version.as_deref(), Some("1.2.3.4"));
        assert_eq!(manifest.display_name.as_deref(), Some("DazzCal"));
        assert_eq!(
            manifest.publisher_display_name.as_deref(),
            Some("Dazz Labs")
        );
        assert_eq!(manifest.app_version().as_deref(), Some("1.2.3+4"));
        let err = AppxManifest::parse("<Package/>").unwrap_err().to_string();
        assert!(err.contains("<Identity>"), "{err}");
    }

    #[test]
    fn app_version_drops_a_zero_revision() {
        let version = |v: &str| {
            AppxManifest {
                text: String::new(),
                identity_name: None,
                publisher: None,
                version: Some(v.into()),
                display_name: None,
                publisher_display_name: None,
            }
            .app_version()
        };
        assert_eq!(version("1.2.3.0").as_deref(), Some("1.2.3"));
        assert_eq!(version("1.2.3.4").as_deref(), Some("1.2.3+4"));
        assert_eq!(version("1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(version("1.2").as_deref(), Some("1.2"));
    }

    #[test]
    fn render_substitutes_identity_tokens_and_language() {
        let manifest = AppxManifest::parse(MANIFEST).unwrap();
        let xml = manifest.render(&ManifestIdentity {
            name: "DazzLabs.DazzCal",
            publisher: "CN=Fastforge CI",
            version: "1.2.3.4",
            architecture: "arm64",
            executable: "DazzCal.exe",
        });
        assert!(xml.contains(
            "<Identity\n    Name=\"DazzLabs.DazzCal\"\n    Publisher=\"CN=Fastforge CI\"\n    Version=\"1.2.3.4\" ProcessorArchitecture=\"arm64\"/>"
        ), "{xml}");
        assert!(xml.contains(
            r#"<Application Id="App" Executable="DazzCal.exe" EntryPoint="Windows.FullTrustApplication">"#
        ));
        assert!(xml.contains(r#"<Resource Language="en-us"/>"#));
        // Everything else is kept verbatim, including the comment.
        assert!(xml.contains("<!-- <Identity Name=\"commented.out\""));
        assert!(xml.contains(r#"<rescap:Capability Name="runFullTrust" />"#));
        assert!(xml.contains(r#"Wide310x150Logo="Assets\Wide310x150Logo.png""#));
    }

    #[test]
    fn render_replaces_an_existing_architecture_and_escapes_values() {
        let manifest = AppxManifest::parse(
            r#"<Package><Identity Name='a.b' Publisher='CN=X' Version='1.0.0.0' ProcessorArchitecture='x64' /><Application Executable="$targetnametoken$.exe"/></Package>"#,
        )
        .unwrap();
        let xml = manifest.render(&ManifestIdentity {
            name: "a.b",
            publisher: "CN=Me & You",
            version: "2.0.0.0",
            architecture: "arm64",
            executable: "My App.exe",
        });
        assert_eq!(
            xml,
            r#"<Package><Identity Name='a.b' Publisher='CN=Me &amp; You' Version='2.0.0.0' ProcessorArchitecture='arm64' /><Application Executable="My App.exe"/></Package>"#
        );
    }

    #[test]
    fn attribute_helpers() {
        assert_eq!(
            attributes(r#"<Identity Name="x" Version='1'/>"#),
            [
                ("Name".to_string(), "x".to_string()),
                ("Version".to_string(), "1".to_string())
            ]
        );
        assert_eq!(
            set_attribute("<Identity/>", "Name", "x"),
            r#"<Identity Name="x"/>"#
        );
        assert_eq!(
            set_attribute(r#"<Identity Name="old" >"#, "Name", "new"),
            r#"<Identity Name="new" >"#
        );
        // `Version` must not match inside `MinVersion`.
        assert_eq!(
            set_attribute(r#"<T MinVersion="1" />"#, "Version", "2"),
            r#"<T MinVersion="1" Version="2"/>"#
        );
        assert_eq!(
            element_text("<a><b>x &amp; y</b></a>", "b").as_deref(),
            Some("x & y")
        );
        assert_eq!(element_text("<a><b><c/></b></a>", "b"), None);
        assert_eq!(element_text("<a><b/></a>", "b"), None);
    }
}
