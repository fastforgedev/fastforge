use anyhow::{Result, anyhow};
use clap::{Args, ValueEnum};
use fastforge_app_analyzer::{
    AnalyzeConfig, AndroidAabAnalyzer, AndroidApkAnalyzer, AppAnalyzer, IOSIpaAnalyzer,
    MacOSAppAnalyzer, MacOSDmgAnalyzer,
};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Artifact formats the analyzers understand. `.app` is a directory rather
/// than a file, which is why scanning has to know about it too.
const SUPPORTED_EXTENSIONS: &[&str] = &["apk", "aab", "ipa", "dmg", "app"];

/// Guards against walking a pathological directory tree forever.
const MAX_SCAN_DEPTH: usize = 16;

/// Ceiling on concurrent analyses, so scanning a big directory does not spawn
/// an unbounded number of helper processes.
const MAX_ANALYSIS_THREADS: usize = 8;

#[derive(Args)]
pub struct AnalyzeArgs {
    /// Artifacts to analyze. A directory is scanned for supported packages.
    #[arg(value_name = "PATH", required = true, num_args = 1..)]
    pub paths: Vec<String>,
    #[arg(short, long = "output")]
    pub output: Option<String>,
    /// Output format. Defaults to the one implied by `--output`, else JSON.
    #[arg(long, value_enum)]
    pub format: Option<ReportFormat>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReportFormat {
    Json,
    Html,
}

/// An artifact to analyze, and whether the user named it directly.
struct Artifact {
    path: PathBuf,
    explicit: bool,
}

/// One analyzed artifact, or the reason it could not be analyzed.
struct Analysis {
    path: PathBuf,
    result: Result<Value>,
}

pub async fn execute(args: &AnalyzeArgs) -> Result<()> {
    log::info!("Executing analyze command");

    let artifacts = discover(&args.paths)?;
    if artifacts.is_empty() {
        return Err(anyhow!(
            "No supported packages found. Supported formats: {}",
            SUPPORTED_EXTENSIONS
                .iter()
                .map(|extension| format!(".{}", extension))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let analyses = analyze_all(&artifacts)?;
    let format = resolve_format(args);
    let rendered = match format {
        // The report always carries the whole result set; the flat shape below
        // exists only for the JSON contract.
        ReportFormat::Html => render_report(&Value::Object(full_payload(&analyses))),
        ReportFormat::Json => serde_json::to_string_pretty(&json_payload(&artifacts, &analyses))?,
    };

    match &args.output {
        Some(output) => {
            std::fs::write(output, &rendered)?;
            if format == ReportFormat::Html {
                eprintln!("Report written to {}", output);
            }
        }
        None => println!("{}", rendered),
    }

    Ok(())
}

/// Falls back to the format implied by the output file's extension, so
/// `--output report.html` does the obvious thing.
fn resolve_format(args: &AnalyzeArgs) -> ReportFormat {
    if let Some(format) = args.format {
        return format;
    }
    let extension = args
        .output
        .as_deref()
        .and_then(|output| Path::new(output).extension())
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("html" | "htm") => ReportFormat::Html,
        _ => ReportFormat::Json,
    }
}

fn analyze_all(artifacts: &[Artifact]) -> Result<Vec<Analysis>> {
    let results = match artifacts {
        [artifact] => vec![analyze_artifact(&artifact.path)],
        artifacts => analyze_in_parallel(artifacts),
    };

    let analyses: Vec<Analysis> = artifacts
        .iter()
        .zip(results)
        .map(|(artifact, result)| Analysis {
            path: artifact.path.clone(),
            result,
        })
        .collect();

    // A path the user named has to work; one that merely turned up during a
    // scan is reported as a failure so the rest of the run still completes.
    for (artifact, analysis) in artifacts.iter().zip(&analyses) {
        if artifact.explicit
            && let Err(error) = &analysis.result
        {
            return Err(anyhow!("{}: {}", artifact.path.display(), error));
        }
    }

    Ok(analyses)
}

/// Analyzing an artifact is mostly spent waiting — on `codesign`, `spctl`,
/// `aapt2`, or on walking a bundle — so a directory full of them is worth
/// spreading across threads. Results stay in the order the artifacts were
/// discovered; only the progress lines arrive as each one finishes.
fn analyze_in_parallel(artifacts: &[Artifact]) -> Vec<Result<Value>> {
    let total = artifacts.len();
    let workers = std::thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(4)
        .min(MAX_ANALYSIS_THREADS)
        .min(total);

    let next = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Result<Value>>>> = Mutex::new((0..total).map(|_| None).collect());

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(artifact) = artifacts.get(index) else {
                        break;
                    };

                    let result = analyze_artifact(&artifact.path);
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    eprintln!("[{}/{}] {}", done, total, artifact.path.display());
                    results.lock().expect("results lock")[index] = Some(result);
                }
            });
        }
    });

    results
        .into_inner()
        .expect("results lock")
        .into_iter()
        .map(|result| result.expect("every artifact is analyzed exactly once"))
        .collect()
}

fn analyze_artifact(path: &Path) -> Result<Value> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    let config = AnalyzeConfig::new(path.to_string_lossy().into_owned());
    let result = match extension.as_deref() {
        Some("apk") => AndroidApkAnalyzer::new().analyze(config)?,
        Some("aab") => AndroidAabAnalyzer::new().analyze(config)?,
        Some("ipa") => IOSIpaAnalyzer::new().analyze(config)?,
        Some("dmg") => MacOSDmgAnalyzer::new().analyze(config)?,
        Some("app") => MacOSAppAnalyzer::new().analyze(config)?,
        Some(extension) => return Err(anyhow!("Unsupported file extension: .{}", extension)),
        None => return Err(anyhow!("Unable to determine file extension")),
    };
    Ok(result.data)
}

/// Analyzing one named file keeps returning just that artifact's payload;
/// anything else is wrapped so the results stay distinguishable.
fn json_payload(artifacts: &[Artifact], analyses: &[Analysis]) -> Value {
    if let [artifact] = artifacts
        && artifact.explicit
        && let [analysis] = analyses
        && let Ok(data) = &analysis.result
    {
        return data.clone();
    }
    Value::Object(full_payload(analyses))
}

/// Every analyzed artifact plus whatever failed, which is what the HTML report
/// renders from and what the JSON output carries for a multi-artifact run.
fn full_payload(analyses: &[Analysis]) -> Map<String, Value> {
    let mut succeeded = Vec::new();
    let mut failures = Vec::new();
    for analysis in analyses {
        match &analysis.result {
            Ok(data) => succeeded.push(data.clone()),
            Err(error) => failures.push(json!({
                "path": analysis.path.to_string_lossy(),
                "error": error.to_string(),
            })),
        }
    }

    let mut payload = Map::new();
    payload.insert(
        "generatedAt".to_string(),
        Value::String(chrono::Local::now().to_rfc3339()),
    );
    payload.insert("artifactCount".to_string(), json!(succeeded.len()));
    payload.insert("artifacts".to_string(), Value::Array(succeeded));
    if !failures.is_empty() {
        payload.insert("failures".to_string(), Value::Array(failures));
    }
    payload
}

// ── Discovery ─────────────────────────────────────────────────────────────────

/// Resolves the given paths into artifacts, scanning any directory for the
/// supported package formats.
fn discover(paths: &[String]) -> Result<Vec<Artifact>> {
    let mut artifacts = Vec::new();
    let mut seen = HashSet::new();

    for path in paths {
        let path = path.trim();
        if path.is_empty() {
            return Err(anyhow!("Analyze path cannot be empty"));
        }
        let path = PathBuf::from(path);

        if !path.exists() {
            return Err(anyhow!("Path not found: {}", path.display()));
        }
        if is_artifact(&path) {
            push_unique(&mut artifacts, &mut seen, path, true);
            continue;
        }
        if path.is_dir() {
            let mut found = Vec::new();
            scan(&path, 0, &mut found);
            found.sort();
            for artifact in found {
                push_unique(&mut artifacts, &mut seen, artifact, false);
            }
            continue;
        }

        // A named file the analyzers cannot handle is a mistake worth reporting.
        return Err(
            match path.extension().and_then(|extension| extension.to_str()) {
                Some(extension) => anyhow!("Unsupported file extension: .{}", extension),
                None => anyhow!("Unable to determine file extension"),
            },
        );
    }

    Ok(artifacts)
}

fn push_unique(
    artifacts: &mut Vec<Artifact>,
    seen: &mut HashSet<PathBuf>,
    path: PathBuf,
    explicit: bool,
) {
    let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    if seen.insert(key) {
        artifacts.push(Artifact { path, explicit });
    }
}

/// Collects artifacts under `dir`, skipping hidden entries so a scan does not
/// wander into `.git` and the like.
fn scan(dir: &Path, depth: usize, artifacts: &mut Vec<PathBuf>) {
    if depth >= MAX_SCAN_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if is_hidden(&path) {
            continue;
        }

        // A symlinked artifact is worth analyzing — `/Applications/Safari.app`
        // is one. Symlinked directories are never descended into, which is
        // what keeps a scan from looping.
        if file_type.is_symlink() {
            if is_artifact(&path) {
                artifacts.push(path);
            }
            continue;
        }

        if is_artifact(&path) {
            // A `.app` bundle is an artifact, not a directory to descend into.
            artifacts.push(path);
        } else if file_type.is_dir() {
            scan(&path, depth + 1, artifacts);
        }
    }
}

fn is_artifact(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    let extension = extension.to_ascii_lowercase();
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return false;
    }
    // `.app` is a bundle directory; every other format is a plain file.
    if extension == "app" {
        path.is_dir()
    } else {
        path.is_file()
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
}

// ── Report ────────────────────────────────────────────────────────────────────

/// The report is a static page that renders itself from the analysis embedded
/// in it: the summary, the charts, sorting and the expandable rows all read the
/// same JSON. Generating it is therefore splicing one value into one template —
/// no markup is assembled here, and the page can be edited as a page.
const TEMPLATE: &str = include_str!("../../../assets/report.html");

/// Where the analysis is spliced in. It appears exactly once in the template
/// (enforced by a test), so a stray second occurrence cannot swallow the data.
const DATA_PLACEHOLDER: &str = "__FASTFORGE_ANALYSIS_DATA__";

/// Renders a standalone HTML report for the analyzed artifacts.
fn render_report(payload: &Value) -> String {
    let data = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    TEMPLATE.replacen(DATA_PLACEHOLDER, &escape_for_script(&data), 1)
}

/// Keeps the embedded JSON from ending the `<script>` block that carries it.
///
/// Every `<` in serialized JSON sits inside a string, so escaping it to `<`
/// leaves the value identical while making `</script>` — and the `<!--` that
/// would open an HTML comment — impossible to write, whether by accident or
/// through a crafted app name.
fn escape_for_script(json: &str) -> String {
    json.replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn template_carries_exactly_one_placeholder() {
        assert_eq!(
            TEMPLATE.matches(DATA_PLACEHOLDER).count(),
            1,
            "a second placeholder would be replaced by the analysis too, breaking the page"
        );
    }

    #[test]
    fn rendering_embeds_the_payload_and_consumes_the_placeholder() {
        let html = render_report(&json!({ "artifacts": [{ "name": "Example" }] }));

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains(r#"{"artifacts":[{"name":"Example"}]}"#));
        assert!(!html.contains(DATA_PLACEHOLDER));
    }

    #[test]
    fn embedded_data_cannot_end_the_script_block() {
        // An app name is read out of a third-party package, so it is exactly the
        // kind of value that could try to close the tag carrying it.
        let html = render_report(&json!({ "artifacts": [{ "name": "</script><img src=x>" }] }));

        assert!(!html.contains("</script><img"));
        // One block for the data, one for the page's own script.
        assert_eq!(html.matches("<script").count(), 2);
    }
}
