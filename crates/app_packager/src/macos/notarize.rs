//! Notarization with `xcrun notarytool` and stapling with `xcrun stapler`,
//! shared by the DMG, PKG and ZIP packagers.
//!
//! The order matters: the `.app` is notarized and stapled before it goes
//! into a DMG or ZIP, so the archive ships a stapled app; the DMG and PKG are
//! then notarized and stapled on their own (a ZIP cannot be stapled).

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use fastforge_core::{PackageConfig, PackageError};
use serde::Deserialize;

/// Packaging variable that turns notarization on for every macOS DMG, PKG and
/// ZIP target (set by `fastforge package --notarize`).
pub const NOTARIZE_ENV: &str = "FASTFORGE_NOTARIZE";

const KEYCHAIN_PROFILE_ENV: &str = "NOTARYTOOL_KEYCHAIN_PROFILE";
const KEY_ID_ENV: &str = "APP_STORE_CONNECT_KEY_ID";
const ISSUER_ID_ENV: &str = "APP_STORE_CONNECT_ISSUER_ID";
const KEY_PATH_ENV: &str = "APP_STORE_CONNECT_KEY_PATH";
const APPLE_ID_ENV: &str = "APPLE_ID";
const PASSWORD_ENV: &str = "APPLE_APP_SPECIFIC_PASSWORD";
const TEAM_ID_ENV: &str = "APPLE_TEAM_ID";

/// Whether a packager should notarize: its `make_config.yaml` asks for it, or
/// the `FASTFORGE_NOTARIZE` packaging variable is true.
pub fn enabled(configured: bool, config: &PackageConfig) -> bool {
    configured
        || config
            .env_var(NOTARIZE_ENV)
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "yes"))
}

/// How `notarytool` authenticates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credentials {
    /// A profile stored with `xcrun notarytool store-credentials`.
    KeychainProfile(String),
    /// An App Store Connect API key (`.p8`). Individual keys have no issuer.
    ApiKey {
        key_path: String,
        key_id: String,
        issuer_id: Option<String>,
    },
    /// Apple ID with an app-specific password.
    AppleId {
        apple_id: String,
        password: String,
        team_id: String,
    },
}

impl Credentials {
    /// Resolves the credentials from the packaging variables (falling back to
    /// the process environment), preferring a keychain profile, then an App
    /// Store Connect API key, then an Apple ID.
    pub fn from_config(config: &PackageConfig) -> Result<Self, PackageError> {
        Self::resolve(|key| config.env_var(key))
    }

    fn resolve(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, PackageError> {
        if let Some(profile) = lookup(KEYCHAIN_PROFILE_ENV) {
            return Ok(Self::KeychainProfile(profile));
        }
        let key_id = lookup(KEY_ID_ENV);
        let key_path = lookup(KEY_PATH_ENV);
        if key_id.is_some() || key_path.is_some() {
            return match (key_id, key_path) {
                (Some(key_id), Some(key_path)) => Ok(Self::ApiKey {
                    key_path,
                    key_id,
                    issuer_id: lookup(ISSUER_ID_ENV),
                }),
                (None, _) => Err(missing(&[KEY_ID_ENV])),
                (_, None) => Err(missing(&[KEY_PATH_ENV])),
            };
        }
        let apple_id = lookup(APPLE_ID_ENV);
        let password = lookup(PASSWORD_ENV);
        let team_id = lookup(TEAM_ID_ENV);
        if apple_id.is_some() || password.is_some() || team_id.is_some() {
            return match (apple_id, password, team_id) {
                (Some(apple_id), Some(password), Some(team_id)) => Ok(Self::AppleId {
                    apple_id,
                    password,
                    team_id,
                }),
                (apple_id, password, team_id) => {
                    let names = [
                        (APPLE_ID_ENV, apple_id.is_none()),
                        (PASSWORD_ENV, password.is_none()),
                        (TEAM_ID_ENV, team_id.is_none()),
                    ];
                    let unset: Vec<&str> = names
                        .iter()
                        .filter(|(_, unset)| *unset)
                        .map(|(name, _)| *name)
                        .collect();
                    Err(missing(&unset))
                }
            };
        }
        Err(PackageError::General(format!(
            "Notarization needs credentials: set {KEYCHAIN_PROFILE_ENV} (a profile saved with \
             `xcrun notarytool store-credentials`), or {KEY_ID_ENV}, {ISSUER_ID_ENV} and \
             {KEY_PATH_ENV} (an App Store Connect API key), or {APPLE_ID_ENV}, {PASSWORD_ENV} \
             and {TEAM_ID_ENV}."
        )))
    }

    /// The `notarytool` authentication arguments.
    fn args(&self) -> Vec<OsString> {
        let pairs: Vec<(&str, &str)> = match self {
            Self::KeychainProfile(profile) => vec![("--keychain-profile", profile)],
            Self::ApiKey {
                key_path,
                key_id,
                issuer_id,
            } => {
                let mut pairs = vec![("--key", key_path.as_str()), ("--key-id", key_id)];
                if let Some(issuer_id) = issuer_id {
                    pairs.push(("--issuer", issuer_id));
                }
                pairs
            }
            Self::AppleId {
                apple_id,
                password,
                team_id,
            } => vec![
                ("--apple-id", apple_id),
                ("--password", password),
                ("--team-id", team_id),
            ],
        };
        pairs
            .into_iter()
            .flat_map(|(flag, value)| [flag.into(), value.into()])
            .collect()
    }
}

fn missing(names: &[&str]) -> PackageError {
    PackageError::General(format!(
        "Notarization credentials are incomplete: {} not set.",
        names.join(", ")
    ))
}

/// Notarizes and staples the `.app` in place, unless it already carries a
/// valid ticket (for example when an earlier target of the same build
/// notarized it). `notarytool` takes a zip of the bundle, created with
/// `ditto` so the bundle's metadata survives.
pub fn notarize_app(
    app: &Path,
    credentials: &Credentials,
    work_dir: &Path,
) -> Result<(), PackageError> {
    if is_stapled(app) {
        eprintln!("{} is already notarized and stapled.", app.display());
        return Ok(());
    }
    check_app_signature(app)?;
    let stem = app.file_stem().unwrap_or_default().to_string_lossy();
    let archive = work_dir.join(format!("{stem}-notarization.zip"));
    run(Command::new("ditto").args([
        "-c".as_ref(),
        "-k".as_ref(),
        "--keepParent".as_ref(),
        app.as_os_str(),
        archive.as_os_str(),
    ]))?;
    let submitted = submit(&archive, app, credentials);
    std::fs::remove_file(&archive).ok();
    submitted?;
    staple(app)
}

/// Notarizes a signed DMG or PKG and staples the ticket to it.
pub fn notarize_and_staple(file: &Path, credentials: &Credentials) -> Result<(), PackageError> {
    submit(file, file, credentials)?;
    staple(file)
}

/// `xcrun notarytool submit --wait`; on any status but `Accepted`, fetches
/// the notary log and fails with its issues. `subject` is what the messages
/// name (the `.app` rather than the zip submitted for it).
fn submit(file: &Path, subject: &Path, credentials: &Credentials) -> Result<(), PackageError> {
    eprintln!(
        "Notarizing {} (this usually takes a few minutes)...",
        subject.display()
    );
    let mut args: Vec<OsString> = vec![
        "notarytool".into(),
        "submit".into(),
        file.into(),
        "--wait".into(),
        "--output-format".into(),
        "json".into(),
    ];
    args.extend(credentials.args());
    let output = Command::new("xcrun")
        .args(&args)
        .output()
        .map_err(|e| PackageError::MissingTool(format!("xcrun: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(submission) = parse_submission(&stdout) else {
        return Err(PackageError::CommandFailed {
            command: "xcrun notarytool submit".into(),
            stderr: format!("{}{}", String::from_utf8_lossy(&output.stderr), stdout),
        });
    };
    if submission.status.as_deref() == Some("Accepted") {
        eprintln!(
            "Notarization accepted for {} (submission {}).",
            subject.display(),
            submission.id
        );
        return Ok(());
    }
    let mut message = format!(
        "Notarization of {} finished with status {} (submission {})",
        subject.display(),
        submission.status.as_deref().unwrap_or("unknown"),
        submission.id
    );
    if let Some(detail) = submission.message.filter(|m| !m.is_empty()) {
        message.push_str(&format!(": {detail}"));
    }
    match fetch_log(&submission.id, credentials) {
        Some(log) => message.push_str(&format!("\n{log}")),
        None => message.push_str(&format!(
            "\nRun `xcrun notarytool log {}` for details.",
            submission.id
        )),
    }
    Err(PackageError::General(message))
}

#[derive(Debug, Deserialize)]
struct Submission {
    id: String,
    status: Option<String>,
    message: Option<String>,
}

/// Parses `notarytool submit --output-format json`, which prints one JSON
/// object (possibly after progress lines).
fn parse_submission(stdout: &str) -> Option<Submission> {
    let start = stdout.find('{')?;
    let end = stdout.rfind('}')?;
    serde_json::from_str(stdout.get(start..=end)?).ok()
}

/// Fetches the notary log and formats its issues; `None` when it can't be
/// read.
fn fetch_log(id: &str, credentials: &Credentials) -> Option<String> {
    let mut args: Vec<OsString> = vec!["notarytool".into(), "log".into(), id.into()];
    args.extend(credentials.args());
    let output = Command::new("xcrun").args(&args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(format_log(&String::from_utf8_lossy(&output.stdout)))
}

/// Lists the issues of a notary log; falls back to the raw log when it is
/// not the expected JSON.
fn format_log(log: &str) -> String {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Log {
        status_summary: Option<String>,
        issues: Option<Vec<Issue>>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Issue {
        severity: Option<String>,
        path: Option<String>,
        message: Option<String>,
        architecture: Option<String>,
    }
    let Ok(parsed) = serde_json::from_str::<Log>(log) else {
        return log.trim().to_string();
    };
    let mut lines = Vec::new();
    if let Some(summary) = parsed.status_summary {
        lines.push(summary);
    }
    for issue in parsed.issues.unwrap_or_default() {
        let mut line = format!(
            "  [{}] {}",
            issue.severity.as_deref().unwrap_or("issue"),
            issue.message.as_deref().unwrap_or_default()
        );
        if let Some(path) = issue.path {
            line.push_str(&format!("\n      {path}"));
            if let Some(arch) = issue.architecture {
                line.push_str(&format!(" ({arch})"));
            }
        }
        lines.push(line);
    }
    lines.join("\n")
}

fn staple(file: &Path) -> Result<(), PackageError> {
    run(Command::new("xcrun").args(["stapler".as_ref(), "staple".as_ref(), file.as_os_str()]))
}

fn is_stapled(file: &Path) -> bool {
    Command::new("xcrun")
        .args([
            "stapler".as_ref(),
            "validate".as_ref(),
            "-q".as_ref(),
            file.as_os_str(),
        ])
        .output()
        .is_ok_and(|output| output.status.success())
}

/// The identity the `.app` is signed with when it is a Developer ID
/// Application identity, used to sign a DMG that names none.
pub fn developer_id_identity(app: &Path) -> Option<String> {
    let output = Command::new("codesign")
        .args(["-dvv".as_ref(), app.as_os_str()])
        .output()
        .ok()?;
    let details = String::from_utf8_lossy(&output.stderr);
    first_authority(&details)
        .filter(|authority| authority.starts_with("Developer ID Application"))
        .map(str::to_string)
}

fn first_authority(details: &str) -> Option<&str> {
    details
        .lines()
        .find_map(|line| line.trim().strip_prefix("Authority="))
}

/// Checks before submitting what the notary service would reject anyway: an
/// app that isn't signed with a Developer ID Application identity, with the
/// hardened runtime and a secure timestamp.
pub fn check_app_signature(app: &Path) -> Result<(), PackageError> {
    let output = Command::new("codesign")
        .args(["-dvv".as_ref(), app.as_os_str()])
        .output()
        .map_err(|e| PackageError::MissingTool(format!("codesign: {e}")))?;
    // codesign prints the details to stderr.
    let details = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(PackageError::General(format!(
            "{} is not code signed; notarization needs it signed with a \"Developer ID \
             Application\" identity: {}",
            app.display(),
            details.trim()
        )));
    }
    signature_problem(&details).map_or(Ok(()), |problem| {
        Err(PackageError::General(format!(
            "{} cannot be notarized: {problem}",
            app.display()
        )))
    })
}

/// The first reason `codesign -dvv` output rules out notarization.
fn signature_problem(details: &str) -> Option<&'static str> {
    let lines: Vec<&str> = details.lines().map(str::trim).collect();
    if lines.contains(&"Signature=adhoc") {
        return Some(
            "it is ad hoc signed. Sign it with a \"Developer ID Application\" identity \
             (for Flutter and Xcode projects, set the macOS Release signing in Xcode).",
        );
    }
    if !first_authority(details).is_some_and(|a| a.starts_with("Developer ID Application")) {
        return Some(
            "it is not signed with a \"Developer ID Application\" identity, which \
             distribution outside the Mac App Store requires.",
        );
    }
    let runtime = lines
        .iter()
        .filter(|line| line.starts_with("CodeDirectory "))
        .any(|line| line.contains("(runtime)") || line.contains(",runtime"));
    if !runtime {
        return Some(
            "the hardened runtime is not enabled. Turn on \"Hardened Runtime\" in Xcode \
             (ENABLE_HARDENED_RUNTIME = YES) or sign with `codesign --options runtime`.",
        );
    }
    let timestamped = lines
        .iter()
        .filter_map(|line| line.strip_prefix("Timestamp="))
        .any(|value| value != "none");
    if !timestamped {
        return Some(
            "the signature has no secure timestamp. Sign with `codesign --timestamp` \
             (in Xcode, add `--timestamp` to OTHER_CODE_SIGN_FLAGS).",
        );
    }
    None
}

fn run(cmd: &mut Command) -> Result<(), PackageError> {
    let out = cmd.output().map_err(|e| {
        PackageError::MissingTool(format!("{}: {}", cmd.get_program().to_string_lossy(), e))
    })?;
    if !out.status.success() {
        let args: Vec<String> = cmd
            .get_args()
            .take(2)
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        return Err(PackageError::CommandFailed {
            command: format!("{} {}", cmd.get_program().to_string_lossy(), args.join(" ")),
            stderr: format!(
                "{}{}",
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn resolve(vars: &[(&str, &str)]) -> Result<Credentials, PackageError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Credentials::resolve(|key| vars.get(key).cloned())
    }

    fn args(credentials: &Credentials) -> Vec<String> {
        credentials
            .args()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn keychain_profile_wins() {
        let credentials = resolve(&[
            (KEYCHAIN_PROFILE_ENV, "notary"),
            (KEY_ID_ENV, "KEY"),
            (KEY_PATH_ENV, "/k.p8"),
        ])
        .unwrap();
        assert_eq!(args(&credentials), ["--keychain-profile", "notary"]);
    }

    #[test]
    fn api_key_issuer_is_optional() {
        let team = resolve(&[
            (KEY_ID_ENV, "KEY"),
            (ISSUER_ID_ENV, "ISSUER"),
            (KEY_PATH_ENV, "/k.p8"),
        ])
        .unwrap();
        assert_eq!(
            args(&team),
            ["--key", "/k.p8", "--key-id", "KEY", "--issuer", "ISSUER"]
        );
        let individual = resolve(&[(KEY_ID_ENV, "KEY"), (KEY_PATH_ENV, "/k.p8")]).unwrap();
        assert_eq!(args(&individual), ["--key", "/k.p8", "--key-id", "KEY"]);
    }

    #[test]
    fn apple_id_needs_all_three() {
        let credentials = resolve(&[
            (APPLE_ID_ENV, "dev@example.com"),
            (PASSWORD_ENV, "abcd-efgh"),
            (TEAM_ID_ENV, "TEAMID"),
        ])
        .unwrap();
        assert_eq!(
            args(&credentials),
            [
                "--apple-id",
                "dev@example.com",
                "--password",
                "abcd-efgh",
                "--team-id",
                "TEAMID"
            ]
        );
        let error = resolve(&[(APPLE_ID_ENV, "dev@example.com")])
            .unwrap_err()
            .to_string();
        assert!(error.contains(PASSWORD_ENV) && error.contains(TEAM_ID_ENV));
    }

    #[test]
    fn partial_api_key_names_the_missing_variable() {
        let error = resolve(&[(KEY_ID_ENV, "KEY")]).unwrap_err().to_string();
        assert!(error.contains(KEY_PATH_ENV));
    }

    #[test]
    fn no_credentials_lists_the_options() {
        let error = resolve(&[]).unwrap_err().to_string();
        assert!(error.contains(KEYCHAIN_PROFILE_ENV));
        assert!(error.contains(KEY_ID_ENV));
        assert!(error.contains(APPLE_ID_ENV));
    }

    #[test]
    fn submission_json_is_parsed() {
        let stdout = "Conducting pre-submission checks...\n{\"id\":\"2efe2717\",\"status\":\"Invalid\",\"message\":\"Processing complete\"}\n";
        let submission = parse_submission(stdout).unwrap();
        assert_eq!(submission.id, "2efe2717");
        assert_eq!(submission.status.as_deref(), Some("Invalid"));
        assert!(parse_submission("Error: HTTP status code: 401").is_none());
    }

    #[test]
    fn log_issues_are_listed() {
        let log = r#"{
  "status": "Invalid",
  "statusSummary": "Archive contains critical validation errors",
  "issues": [
    {
      "severity": "error",
      "path": "Demo.zip/Demo.app/Contents/MacOS/Demo",
      "message": "The executable does not have the hardened runtime enabled.",
      "architecture": "arm64"
    }
  ]
}"#;
        assert_eq!(
            format_log(log),
            "Archive contains critical validation errors\n  [error] The executable does not have the hardened runtime enabled.\n      Demo.zip/Demo.app/Contents/MacOS/Demo (arm64)"
        );
        assert_eq!(format_log("not json\n"), "not json");
    }

    const SIGNED: &str = "Executable=/build/Demo.app/Contents/MacOS/Demo
Identifier=com.example.demo
Format=app bundle with Mach-O universal (x86_64 arm64)
CodeDirectory v=20500 size=1234 flags=0x10000(runtime) hashes=27+7 location=embedded
Signature size=9045
Authority=Developer ID Application: Example (TEAMID)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=13 Oct 2026 at 10:00:00
TeamIdentifier=TEAMID
";

    #[test]
    fn developer_id_with_runtime_and_timestamp_passes() {
        assert_eq!(signature_problem(SIGNED), None);
    }

    #[test]
    fn signature_problems_are_reported() {
        let adhoc = "Identifier=demo\nCodeDirectory v=20400 size=1 flags=0x20002(adhoc,linker-signed) hashes=1+0\nSignature=adhoc\n";
        assert!(signature_problem(adhoc).unwrap().contains("ad hoc"));

        let development = SIGNED.replace(
            "Authority=Developer ID Application: Example (TEAMID)",
            "Authority=Apple Development: Example (TEAMID)",
        );
        assert!(
            signature_problem(&development)
                .unwrap()
                .contains("Developer ID Application")
        );

        let no_runtime = SIGNED.replace("flags=0x10000(runtime)", "flags=0x0(none)");
        assert!(
            signature_problem(&no_runtime)
                .unwrap()
                .contains("hardened runtime")
        );

        let no_timestamp = SIGNED.replace(
            "Timestamp=13 Oct 2026 at 10:00:00",
            "Signed Time=13 Oct 2026 at 10:00:00",
        );
        assert!(
            signature_problem(&no_timestamp)
                .unwrap()
                .contains("secure timestamp")
        );
    }
}
