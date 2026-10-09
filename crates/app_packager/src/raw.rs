//! Raw file discovery and rendered overlays shared by desktop packagers.

use fastforge_core::{PackageError, Variables, render_variables};
use std::path::{Path, PathBuf};

/// The only file directly in `dir` whose name ends with one of `suffixes`;
/// an error when there are several.
pub(crate) fn single_file_with_suffix(
    dir: &Path,
    suffixes: &[&str],
) -> Result<Option<PathBuf>, PackageError> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            path.is_file() && suffixes.iter().any(|suffix| name.ends_with(suffix))
        })
        .collect();
    found.sort();
    if found.len() > 1 {
        let names: Vec<String> = found
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        return Err(PackageError::General(format!(
            "{} has several {} files ({}); keep only one",
            dir.display(),
            suffixes.join(" or "),
            names.join(", ")
        )));
    }
    Ok(found.pop())
}

pub(crate) fn copy_rendered_tree(
    src: &Path,
    dest: &Path,
    variables: &Variables,
    only: Option<&[&str]>,
) -> Result<(), PackageError> {
    std::fs::create_dir_all(dest)?;
    let mut entries: Vec<_> = std::fs::read_dir(src)?.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = render_variables(&entry.file_name().to_string_lossy(), variables);
        if only.is_some_and(|only| !only.contains(&name.as_str())) {
            continue;
        }
        let from = entry.path();
        let to = dest.join(name);
        let meta = std::fs::symlink_metadata(&from)?;
        if meta.file_type().is_symlink() {
            if to.symlink_metadata().is_ok() {
                std::fs::remove_file(&to)?;
            }
            #[cfg(unix)]
            {
                let target = std::fs::read_link(&from)?;
                let target = render_variables(&target.to_string_lossy(), variables);
                std::os::unix::fs::symlink(target, &to)?;
            }
            #[cfg(not(unix))]
            std::fs::copy(&from, &to).map(|_| ())?;
        } else if meta.is_dir() {
            copy_rendered_tree(&from, &to, variables, None)?;
        } else {
            let bytes = std::fs::read(&from)?;
            match std::str::from_utf8(&bytes) {
                Ok(text) if !bytes.contains(&0) => {
                    std::fs::write(&to, render_variables(text, variables))?
                }
                _ => std::fs::write(&to, &bytes)?,
            }
            std::fs::set_permissions(&to, meta.permissions())?;
        }
    }
    Ok(())
}
