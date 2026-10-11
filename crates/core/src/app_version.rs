//! Flutter/publisher app versions. Package-manager versions are converted separately.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppVersion {
    pub text: String,
    pub core: String,
    /// Includes the pre-release suffix, but no build metadata.
    pub build_name: String,
    /// Kept as text: Apple build versions may contain dots.
    pub build_number: Option<String>,
}

impl AppVersion {
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let version = semver::Version::parse(text)
            .map_err(|e| format!("Invalid app version `{text}`: {e}. Expected major.minor.patch[-prerelease][+build]."))?;
        Ok(Self {
            text: text.to_string(),
            core: format!("{}.{}.{}", version.major, version.minor, version.patch),
            build_name: text.split('+').next().unwrap_or(text).to_string(),
            build_number: (!version.build.is_empty()).then(|| version.build.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_missing_build_from_dotted_build() {
        assert_eq!(AppVersion::parse("1.2.3").unwrap().build_number, None);
        let version = AppVersion::parse(" 1.2.3-beta.1+7.8 ").unwrap();
        assert_eq!(version.text, "1.2.3-beta.1+7.8");
        assert_eq!(version.build_name, "1.2.3-beta.1");
        assert_eq!(version.core, "1.2.3");
        assert_eq!(version.build_number.as_deref(), Some("7.8"));
    }

    #[test]
    fn rejects_malformed_versions() {
        for text in [
            "",
            "1",
            "1.2",
            "01.2.3",
            "1.2.3+",
            "1.2.3-",
            "1.2.3+4+5",
            "1.2.3-beta..1",
            "1.2.3 beta",
        ] {
            assert!(AppVersion::parse(text).is_err(), "{text}");
        }
    }
}
