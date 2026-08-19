use crate::macos::bundle;
use fastforge_core::{AnalyzeConfig, AnalyzeError, AnalyzeResult, AppAnalyzer};
use serde_json::Value;
use std::path::Path;

pub struct MacOSAppAnalyzer;

impl AppAnalyzer for MacOSAppAnalyzer {
    fn new() -> Self {
        Self
    }

    fn name(&self) -> &str {
        "macos-app"
    }

    fn is_supported_on_current_platform(&self) -> bool {
        cfg!(target_os = "macos")
    }

    fn perform_analyze(&self, config: &AnalyzeConfig) -> Result<AnalyzeResult, AnalyzeError> {
        if !self.is_supported_on_current_platform() {
            return Err(AnalyzeError::General(
                "macOS .app bundle analysis is only supported on macOS.".to_string(),
            ));
        }

        let app_path = Path::new(&config.path);
        bundle::validate(app_path)?;

        let data = bundle::inspect(app_path)?;

        log::info!("macOS app bundle analysis completed for {}", config.path);
        Ok(AnalyzeResult::new(true, Value::Object(data)))
    }
}
