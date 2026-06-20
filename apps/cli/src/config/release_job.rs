use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package step of a release job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseJobPackage {
    pub platform: String,
    pub target: String,
    pub channel: Option<String>,
    /// Build arguments passed to `flutter build`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_args: Option<HashMap<String, serde_yaml::Value>>,
    /// Package lifecycle hooks, e.g. `{ "pre": "echo before", "post": ["cmd1", "cmd2"] }`.
    /// Values can be a single string or a list of strings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HashMap<String, serde_yaml::Value>>,
}

/// Publish step of a release job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseJobPublish {
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<HashMap<String, serde_yaml::Value>>,
}

/// A single job inside a release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseJob {
    pub name: String,
    /// Job-level variables that override release- and global-level variables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
    pub package: ReleaseJobPackage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publish: Option<ReleaseJobPublish>,
    /// Shorthand for specifying a single publish target without extra args.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publish_to: Option<String>,
}

impl ReleaseJob {
    /// Returns the resolved publish target if either `publish` or `publish_to`
    /// is set, otherwise `None`.
    pub fn publish_target(&self) -> Option<&str> {
        self.publish_to
            .as_deref()
            .or_else(|| self.publish.as_ref().map(|p| p.target.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_target_prefers_publish_to() {
        let job = ReleaseJob {
            name: "test".to_string(),
            variables: None,
            package: ReleaseJobPackage {
                platform: "android".to_string(),
                target: "apk".to_string(),
                channel: None,
                build_args: None,
                hooks: None,
            },
            publish: Some(ReleaseJobPublish {
                target: "github".to_string(),
                args: None,
            }),
            publish_to: Some("firebase".to_string()),
        };
        assert_eq!(job.publish_target(), Some("firebase"));
    }

    #[test]
    fn test_publish_target_falls_back_to_publish() {
        let job = ReleaseJob {
            name: "test".to_string(),
            variables: None,
            package: ReleaseJobPackage {
                platform: "ios".to_string(),
                target: "ipa".to_string(),
                channel: None,
                build_args: None,
                hooks: None,
            },
            publish: Some(ReleaseJobPublish {
                target: "github".to_string(),
                args: None,
            }),
            publish_to: None,
        };
        assert_eq!(job.publish_target(), Some("github"));
    }

    #[test]
    fn test_publish_target_none_when_not_set() {
        let job = ReleaseJob {
            name: "test".to_string(),
            variables: None,
            package: ReleaseJobPackage {
                platform: "macos".to_string(),
                target: "dmg".to_string(),
                channel: None,
                build_args: None,
                hooks: None,
            },
            publish: None,
            publish_to: None,
        };
        assert_eq!(job.publish_target(), None);
    }
}
