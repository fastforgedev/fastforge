use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult, Platform};

/// Copies the flutter Linux build output directory directly to the output
/// path without any additional packaging, mirroring Dart's
/// `AppPackageMakerDirect("linux")`.
pub struct LinuxDirectPackager;

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<(), PackageError> {
    let out = std::process::Command::new("cp")
        .args(["-r", &src.display().to_string(), &dst.display().to_string()])
        .output()
        .map_err(|e| PackageError::MissingTool(format!("cp: {}", e)))?;
    if !out.status.success() {
        return Err(PackageError::CommandFailed {
            command: "cp".into(),
            stderr: String::from_utf8_lossy(&out.stderr).into(),
        });
    }
    Ok(())
}

impl AppPackager for LinuxDirectPackager {
    fn name(&self) -> &str {
        "direct"
    }

    fn platform(&self) -> Platform {
        Platform::Linux
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
