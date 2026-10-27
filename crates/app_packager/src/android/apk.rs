use std::path::{Path, PathBuf};

use fastforge_core::{
    AppPackager, PackageConfig, PackageError, PackageResult, Platform, android_apk_abi,
};

/// Copies the first build output file (the `.apk` produced by `flutter build apk`)
/// to the versioned output path, mirroring Dart's `AppPackageMakerApk`.
///
/// When every output is a split APK (`--split-per-abi`), each one is copied,
/// with its ABI appended to the artifact name.
pub struct AndroidApkPackager;

/// `<name>.apk` → `<name>-<abi>.apk`.
fn with_abi(output_file: &Path, abi: &str) -> PathBuf {
    let stem = output_file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = match output_file.extension() {
        Some(ext) => format!("{}-{}.{}", stem, abi, ext.to_string_lossy()),
        None => format!("{}-{}", stem, abi),
    };
    output_file.with_file_name(name)
}

impl AppPackager for AndroidApkPackager {
    fn name(&self) -> &str {
        "apk"
    }

    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn package_format(&self) -> &str {
        "apk"
    }

    fn package(&self, config: &PackageConfig) -> Result<PackageResult, PackageError> {
        let src = config
            .first_build_output_file()
            .ok_or_else(|| PackageError::General("no build output files".into()))?;
        let dst = config.output_file()?;
        let splits: Option<Vec<(&Path, &str)>> = config
            .build_output_files
            .iter()
            .map(|file| android_apk_abi(file).map(|abi| (file.as_path(), abi)))
            .collect();
        let Some(splits) = splits else {
            std::fs::copy(src, &dst)?;
            return config.resolve_result(dst);
        };

        let mut artifacts = Vec::with_capacity(splits.len());
        for (src, abi) in splits {
            let dst = with_abi(&dst, abi);
            std::fs::copy(src, &dst)?;
            artifacts.extend(config.resolve_result(dst)?.artifacts);
        }
        Ok(PackageResult {
            artifacts,
            config: Some(config.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn config(tmp: &Path, build_output_files: Vec<PathBuf>) -> PackageConfig {
        PackageConfig {
            app_name: "myapp".into(),
            app_binary_name: "myapp".into(),
            app_version: "1.0.0".into(),
            build_mode: "release".into(),
            platform: Platform::Android,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: "apk".into(),
            is_installer: false,
            build_output_dir: tmp.to_path_buf(),
            build_output_files,
            output_dir: tmp.to_path_buf(),
            packaging_root: None,
            environment: Default::default(),
        }
    }

    #[test]
    fn copies_apk_to_output() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("app-release.apk");
        std::fs::write(&src, b"fake-apk").unwrap();

        let result = AndroidApkPackager
            .package(&config(tmp.path(), vec![src]))
            .unwrap();
        assert_eq!(result.artifacts.len(), 1);
        assert!(result.artifacts[0].exists());
    }

    #[test]
    fn copies_every_split_apk_with_its_abi() {
        let tmp = TempDir::new().unwrap();
        let files: Vec<PathBuf> = ["arm64-v8a", "armeabi-v7a", "x86_64"]
            .iter()
            .map(|abi| {
                let src = tmp.path().join(format!("app-{}-release.apk", abi));
                std::fs::write(&src, abi.as_bytes()).unwrap();
                src
            })
            .collect();

        let result = AndroidApkPackager
            .package(&config(tmp.path(), files))
            .unwrap();
        let names: Vec<String> = result
            .artifacts
            .iter()
            .map(|a| a.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "myapp-1.0.0-android-arm64-v8a.apk",
                "myapp-1.0.0-android-armeabi-v7a.apk",
                "myapp-1.0.0-android-x86_64.apk",
            ]
        );
        assert_eq!(std::fs::read(&result.artifacts[1]).unwrap(), b"armeabi-v7a");
    }
}
