//! The staged Windows application and the raw files used to package it.

use std::path::{Path, PathBuf};
use std::process::Command;

use fastforge_core::{
    PackageConfig, PackageError, ProjectSettings, Variables, environment_variables,
    render_variables,
};

use crate::fs_util::copy_dir_contents;
use crate::raw::{copy_rendered_tree, single_file_with_suffix};

pub(crate) struct RawPackaging {
    dir: PathBuf,
    pub settings: ProjectSettings,
    pub variables: Variables,
}

impl RawPackaging {
    pub fn load(config: &PackageConfig, root: &Path) -> Result<Self, PackageError> {
        Ok(Self::new(
            Path::new(".fastforge/packaging/windows").join(&config.package_format),
            ProjectSettings::load()?,
            config,
            root,
        ))
    }

    pub fn new(
        dir: PathBuf,
        settings: ProjectSettings,
        config: &PackageConfig,
        root: &Path,
    ) -> Self {
        let mut variables = config.package_variables(&settings);
        let arch = architecture(&config.build_output_dir);
        variables.insert("PACKAGE_ARCH".into(), arch.into());
        variables.insert(
            "ARCH".into(),
            if arch == "arm64" { "aarch64" } else { "x86_64" }.into(),
        );
        variables.insert(
            "PACKAGE_NAME".into(),
            settings
                .package_name
                .clone()
                .unwrap_or_else(|| config.app_name.clone()),
        );
        variables.insert(
            "PACKAGE_VERSION".into(),
            config
                .app_version
                .split('+')
                .next()
                .unwrap_or(&config.app_version)
                .into(),
        );
        variables.insert("PACKAGING_DIRECTORY".into(), absolute(root));
        variables.insert(
            "OUTPUT_ARTIFACT_PATH".into(),
            absolute(&config.output_file()),
        );
        Self {
            dir,
            settings,
            variables,
        }
    }

    pub fn template(&self, suffixes: &[&str]) -> Result<Option<PathBuf>, PackageError> {
        single_file_with_suffix(&self.dir, suffixes)
    }

    pub fn render(&self, path: &Path) -> Result<String, PackageError> {
        Ok(render_variables(
            &std::fs::read_to_string(path)?,
            &self.variables,
        ))
    }

    pub fn apply_env(&self, cmd: &mut Command, config: &PackageConfig) {
        cmd.envs(&config.environment);
        cmd.envs(environment_variables(&self.variables));
    }

    /// Bundle first, then the shared overlay, then the format's own files.
    pub fn stage(&self, config: &PackageConfig, root: &Path) -> Result<(), PackageError> {
        copy_dir_contents(&config.build_output_dir, root)?;
        self.install_overlay(root)
    }

    pub fn install_overlay(&self, root: &Path) -> Result<(), PackageError> {
        if let Some(parent) = self.dir.parent() {
            let shared = parent.join("shared/files");
            if shared.is_dir() {
                copy_rendered_tree(&shared, root, &self.variables, None)?;
            }
        }
        let own = self.dir.join("files");
        if own.is_dir() {
            copy_rendered_tree(&own, root, &self.variables, None)?;
        }
        Ok(())
    }

    /// Project variables are also available to generated defaults; legacy
    /// make_config values still take precedence for the actual metadata.
    pub fn value(&self, key: &str) -> Option<String> {
        self.variables.get(key).filter(|s| !s.is_empty()).cloned()
    }
}

pub(crate) fn absolute(path: &Path) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

pub(crate) fn architecture(build_output_dir: &Path) -> &'static str {
    if build_output_dir
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("arm64"))
    {
        "arm64"
    } else {
        "x64"
    }
}

/// Prefer the build system's binary name over helpers bundled alongside it.
pub(crate) fn find_executable(root: &Path, binary_name: &str) -> String {
    let preferred = format!("{binary_name}.exe");
    if root.join(&preferred).is_file() {
        return preferred;
    }
    let mut exes: Vec<_> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    exes.sort();
    exes.into_iter().next().unwrap_or(preferred)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use fastforge_core::Platform;

    pub fn config(root: &Path, format: &str) -> PackageConfig {
        PackageConfig {
            app_name: "demo".into(),
            app_binary_name: "runner".into(),
            app_version: "1.2.3+4".into(),
            build_mode: "release".into(),
            platform: Platform::Windows,
            flavor: None,
            channel: None,
            artifact_name: None,
            package_format: format.into(),
            is_installer: format == "exe",
            build_output_dir: root.join("build/windows/arm64/runner/Release"),
            build_output_files: vec![],
            output_dir: root.join("dist"),
            environment: Default::default(),
        }
    }

    pub fn write(path: &Path, bytes: impl AsRef<[u8]>) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn stages_bundle_shared_then_format_files_and_renders_names() {
        let tmp = tempfile::tempdir().unwrap();
        let config = config(tmp.path(), "exe");
        let dir = tmp.path().join("packaging/exe");
        let raw = RawPackaging::new(
            dir.clone(),
            ProjectSettings::default(),
            &config,
            &tmp.path().join("stage"),
        );
        write(&config.build_output_dir.join("runner.exe"), b"exe\0");
        write(&config.build_output_dir.join("data/app.txt"), "bundle");
        write(
            &dir.parent().unwrap().join("shared/files/data/app.txt"),
            "shared",
        );
        write(&dir.join("files/data/app.txt"), "${APP_VERSION}");
        write(&dir.join("files/${APP_NAME}.bin"), b"\0${APP_NAME}");
        let stage = tmp.path().join("stage");
        raw.stage(&config, &stage).unwrap();
        assert_eq!(std::fs::read(stage.join("runner.exe")).unwrap(), b"exe\0");
        assert_eq!(
            std::fs::read_to_string(stage.join("data/app.txt")).unwrap(),
            "1.2.3+4"
        );
        assert_eq!(
            std::fs::read(stage.join("demo.bin")).unwrap(),
            b"\0${APP_NAME}"
        );
        assert_eq!(raw.value("PACKAGE_ARCH").as_deref(), Some("arm64"));
    }

    #[test]
    fn several_scripts_are_rejected_and_tool_environment_is_applied() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = config(tmp.path(), "exe");
        config
            .environment
            .insert("TEST_VALUE".into(), "legacy".into());
        let mut settings = ProjectSettings::default();
        settings.env.insert("TEST_VALUE".into(), "project".into());
        let dir = tmp.path().join("exe");
        let raw = RawPackaging::new(dir.clone(), settings, &config, &tmp.path().join("stage"));
        write(&dir.join("a.iss"), "");
        write(&dir.join("b.iss"), "");
        assert!(
            raw.template(&[".iss"])
                .unwrap_err()
                .to_string()
                .contains("several")
        );
        let mut command = Command::new("tool");
        raw.apply_env(&mut command, &config);
        let env: std::collections::BTreeMap<_, _> = command.get_envs().collect();
        assert_eq!(
            env[std::ffi::OsStr::new("TEST_VALUE")],
            Some(std::ffi::OsStr::new("project"))
        );
        assert_eq!(
            env[std::ffi::OsStr::new("PACKAGE_ARCH")],
            Some(std::ffi::OsStr::new("arm64"))
        );
    }
}
