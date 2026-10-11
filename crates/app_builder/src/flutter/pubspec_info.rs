use std::path::Path;

/// Information parsed from a Flutter project's `pubspec.yaml`.
#[derive(Debug, Clone)]
pub struct PubspecInfo {
    pub build_name: String,
    pub build_number: Option<String>,
}

impl PubspecInfo {
    /// Reads and validates Flutter's semantic version. Missing files/versions
    /// have no defaults here; malformed versions are errors.
    pub fn load(path: impl AsRef<Path>) -> Result<Option<Self>, fastforge_core::BuildError> {
        use fastforge_core::BuildError;
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(BuildError::Io(e.to_string())),
        };
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| BuildError::Parse(format!("Invalid pubspec.yaml: {e}")))?;
        let Some(version) = yaml.get("version") else {
            return Ok(None);
        };
        let version = version
            .as_str()
            .ok_or_else(|| BuildError::Parse("pubspec.yaml version must be a string".into()))?;
        Self::from_version(version).map(Some)
    }

    pub fn from_version(version: &str) -> Result<Self, fastforge_core::BuildError> {
        let version = fastforge_core::AppVersion::parse(version)
            .map_err(fastforge_core::BuildError::Parse)?;
        Ok(Self {
            build_name: version.build_name,
            build_number: version.build_number,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_version_without_inventing_a_build_number() {
        let info = PubspecInfo::from_version("1.2.3+45").unwrap();
        assert_eq!(info.build_name, "1.2.3");
        assert_eq!(info.build_number.as_deref(), Some("45"));
        let info = PubspecInfo::from_version("1.2.3").unwrap();
        assert_eq!(info.build_name, "1.2.3");
        assert_eq!(info.build_number, None);
    }
}
