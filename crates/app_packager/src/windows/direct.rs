use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

/// Copies the flutter Windows build output directory directly to the output
/// path without any additional packaging, mirroring Dart's
/// `AppPackageMakerDirect("windows")`.
pub struct WindowsDirectPackager;

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<(), PackageError> {
    let out = std::process::Command::new("xcopy")
        .args([
            "/E",
            "/I",
            "/Q",
            &src.display().to_string(),
            &dst.display().to_string(),
        ])
        .output()
        .map_err(|e| PackageError::MissingTool(format!("xcopy: {}", e)))?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: "xcopy".into(),
            stderr: String::from_utf8_lossy(&out.stderr).into(),
        });
    }
    Ok(())
}

impl AppPackager for WindowsDirectPackager {
    fn name(&self) -> &str {
        "direct"
    }

    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn package_format(&self) -> &str {
        ""
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        // Mirrors Dart: the destination directory uses the rendered artifact
        // name (no extension for direct output).
        let dst = config.output_file();
        copy_dir(&config.build_output_dir, &dst)?;
        Ok(PackageResult {
            artifacts: vec![dst],
        })
    }
}
