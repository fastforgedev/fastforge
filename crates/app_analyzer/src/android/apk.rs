use crate::android::techstack::Layout;
use crate::android::{badging, sdk, signature, techstack};
use crate::archive;
use crate::archive::Archive;
use crate::command;
use crate::json_util;
use fastforge_core::{AnalyzeConfig, AnalyzeError, AnalyzeResult, AppAnalyzer};
use serde_json::{Map, Value};
use std::path::Path;

pub struct AndroidApkAnalyzer;

impl AppAnalyzer for AndroidApkAnalyzer {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        "android-apk"
    }

    fn is_supported_on_current_platform(&self) -> bool {
        true
    }

    fn perform_analyze(&self, config: &AnalyzeConfig) -> Result<AnalyzeResult, AnalyzeError> {
        let apk_path = Path::new(&config.path);
        if !apk_path.is_file() {
            return Err(AnalyzeError::NotFound(format!(
                "APK not found: {}",
                config.path
            )));
        }

        let badging = read_badging(&config.path)?;
        let mut archive = Archive::open(apk_path)?;
        let layout = Layout::apk();

        let mut data = Map::new();
        data.insert("platform".to_string(), Value::String("android".to_string()));
        data.insert("format".to_string(), Value::String("apk".to_string()));
        data.append(&mut badging::identity_fields(&badging.identity));
        data.append(&mut super::artifact_fields(apk_path));

        // aapt2 reports the ABIs the manifest advertises; the `lib/` layout
        // shows what the APK actually carries. They agree for a normal build
        // and diverge for split or stripped APKs.
        let mut abis = badging.abis.clone();
        if abis.is_empty() {
            abis = techstack::native_abis(&archive, &layout);
        }
        json_util::insert_text_array(&mut data, "abis", Some(abis));

        json_util::insert_object(&mut data, "manifest", badging.manifest);
        json_util::insert_object(
            &mut data,
            "techStack",
            techstack::collect(&mut archive, &layout),
        );
        json_util::insert_object(
            &mut data,
            "contents",
            archive::contents_summary(archive.entries(), ""),
        );
        json_util::insert_value(&mut data, "signature", signature::inspect(&config.path));

        log::info!("APK analysis completed for {}", config.path);
        Ok(AnalyzeResult::new(true, Value::Object(data)))
    }
}

fn read_badging(path: &str) -> Result<badging::Badging, AnalyzeError> {
    let aapt2 = sdk::build_tool("aapt2").ok_or_else(|| {
        AnalyzeError::NotFound(
            "aapt2 in Android build-tools (set ANDROID_HOME to your SDK)".to_string(),
        )
    })?;

    let output = command::run(&aapt2.to_string_lossy(), &["dump", "badging", path])
        .ok_or_else(|| AnalyzeError::General("Failed to run aapt2".to_string()))?;
    if !output.success {
        return Err(AnalyzeError::CommandFailed {
            command: "aapt2".to_string(),
            stderr: output.stderr_text(),
        });
    }

    badging::parse(&output.stdout_text())
}
