use crate::config::release_job::ReleaseJob;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A named release that groups one or more jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub name: String,
    /// Release-level variables that override global variables but are
    /// overridden by job-level variables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
    #[serde(default)]
    pub jobs: Vec<ReleaseJob>,
}

impl Release {
    /// Return the subset of jobs selected by `job_names`.  
    /// If `job_names` is empty the full list is returned.
    pub fn filter_jobs(&self, job_names: &[String], skip_names: &[String]) -> Vec<&ReleaseJob> {
        self.jobs
            .iter()
            .filter(|j| {
                if !job_names.is_empty() {
                    return job_names.contains(&j.name);
                }
                if !skip_names.is_empty() {
                    return !skip_names.contains(&j.name);
                }
                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::release_job::{ReleaseJob, ReleaseJobPackage};

    fn make_job(name: &str) -> ReleaseJob {
        ReleaseJob {
            name: name.to_string(),
            variables: None,
            package: ReleaseJobPackage {
                platform: "android".to_string(),
                target: "apk".to_string(),
                channel: None,
                build_args: None,
                hooks: None,
            },
            publish: None,
            publish_to: None,
        }
    }

    fn make_release(names: &[&str]) -> Release {
        Release {
            name: "test-release".to_string(),
            variables: None,
            jobs: names.iter().map(|n| make_job(n)).collect(),
        }
    }

    #[test]
    fn filter_jobs_empty_returns_all() {
        let release = make_release(&["a", "b", "c"]);
        let jobs = release.filter_jobs(&[], &[]);
        assert_eq!(jobs.len(), 3);
    }

    #[test]
    fn filter_jobs_by_name() {
        let release = make_release(&["a", "b", "c"]);
        let jobs = release.filter_jobs(&["a".to_string(), "c".to_string()], &[]);
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].name, "a");
        assert_eq!(jobs[1].name, "c");
    }

    #[test]
    fn filter_jobs_skip_names() {
        let release = make_release(&["a", "b", "c"]);
        let jobs = release.filter_jobs(&[], &["b".to_string()]);
        assert_eq!(jobs.len(), 2);
        assert!(jobs.iter().all(|j| j.name != "b"));
    }

    #[test]
    fn filter_jobs_names_takes_precedence_over_skip() {
        let release = make_release(&["a", "b", "c"]);
        // When `jobs` is specified, `skip-jobs` is ignored (matches Dart behavior).
        let jobs = release.filter_jobs(&["a".to_string()], &["a".to_string()]);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "a");
    }
}
