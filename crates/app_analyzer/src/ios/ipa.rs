use crate::archive::Archive;
use crate::ios::bundle;
use crate::{archive as archive_util, checksum, json_util, plist_util};
use fastforge_core::{AnalyzeConfig, AnalyzeError, AnalyzeResult, AppAnalyzer};
use serde_json::{Map, Value, json};
use std::path::Path;

pub struct IOSIpaAnalyzer;

impl AppAnalyzer for IOSIpaAnalyzer {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        "ios-ipa"
    }

    fn is_supported_on_current_platform(&self) -> bool {
        true
    }

    fn perform_analyze(&self, config: &AnalyzeConfig) -> Result<AnalyzeResult, AnalyzeError> {
        let ipa_path = Path::new(&config.path);
        if !ipa_path.is_file() {
            return Err(AnalyzeError::NotFound(format!(
                "IPA not found: {}",
                config.path
            )));
        }

        let mut archive = Archive::open(ipa_path)?;
        let app = bundle::find(&mut archive)?;

        let identifier = plist_util::require_text(&app.info, "CFBundleIdentifier")?;
        let name = plist_util::text(&app.info, "CFBundleDisplayName")
            .or_else(|| plist_util::text(&app.info, "CFBundleName"))
            .ok_or_else(|| {
                AnalyzeError::Parse(
                    "Missing CFBundleDisplayName/CFBundleName in Info.plist".to_string(),
                )
            })?;
        let version = plist_util::require_text(&app.info, "CFBundleShortVersionString")?;
        let build_number = plist_util::require_text(&app.info, "CFBundleVersion")?;

        let executable = bundle::read_executable(&mut archive, &app);

        let mut data = Map::new();
        data.insert("platform".to_string(), Value::String("ios".to_string()));
        data.insert("format".to_string(), Value::String("ipa".to_string()));
        data.insert("identifier".to_string(), Value::String(identifier));
        data.insert("name".to_string(), Value::String(name));
        data.insert("version".to_string(), Value::String(version));
        data.insert("buildNumber".to_string(), Value::String(build_number));

        json_util::insert_text(
            &mut data,
            "path",
            Some(
                std::fs::canonicalize(ipa_path)
                    .unwrap_or_else(|_| ipa_path.to_path_buf())
                    .to_string_lossy()
                    .into_owned(),
            ),
        );
        json_util::insert_text(
            &mut data,
            "fileName",
            ipa_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
        );
        data.insert(
            "sizeBytes".to_string(),
            json!(
                std::fs::metadata(ipa_path)
                    .map(|meta| meta.len())
                    .unwrap_or(0)
            ),
        );
        json_util::insert_text(&mut data, "sha256", checksum::sha256_of(ipa_path));
        json_util::insert_text(
            &mut data,
            "bundlePath",
            Some(app.prefix.trim_end_matches('/').to_string()),
        );

        data.append(&mut bundle::declared_metadata(&app.info));
        if let Some(executable) = executable.as_ref() {
            json_util::insert_text_array(
                &mut data,
                "architectures",
                Some(executable.architectures.clone()),
            );
        }

        json_util::insert_object(&mut data, "buildInfo", bundle::build_info(&app.info));
        json_util::insert_object(
            &mut data,
            "techStack",
            bundle::tech_stack(&mut archive, &app, executable.as_ref()),
        );
        json_util::insert_object(
            &mut data,
            "components",
            bundle::components(&mut archive, &app),
        );
        json_util::insert_object(&mut data, "capabilities", bundle::capabilities(&app.info));
        json_util::insert_object(
            &mut data,
            "contents",
            archive_util::contents_summary(archive.entries(), &app.prefix),
        );
        json_util::insert_value(
            &mut data,
            "provisioningProfile",
            bundle::provisioning_profile(&mut archive, &app),
        );
        json_util::insert_object(&mut data, "codeSignature", code_signature(&archive, &app));

        log::info!("IPA analysis completed for {}", config.path);
        Ok(AnalyzeResult::new(true, Value::Object(data)))
    }
}

/// An IPA carries a sealed resource directory when it is signed. Verifying that
/// signature would mean unpacking the whole payload, so only its presence and
/// the profile it was signed with are reported.
fn code_signature(archive: &Archive, app: &bundle::AppBundle) -> Map<String, Value> {
    let mut signature = Map::new();
    signature.insert(
        "signed".to_string(),
        Value::Bool(archive.contains_prefix(&format!("{}_CodeSignature/", app.prefix))),
    );
    signature
}
