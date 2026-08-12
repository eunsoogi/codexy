use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::Result;

#[must_use]
pub fn runtime_package_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[must_use]
pub fn repository_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let runtime = runtime_package_root();
        runtime
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_default()
    })
    .as_path()
}

#[must_use]
pub fn plugin_root() -> PathBuf {
    if let Some(path) = std::env::var_os("CODEXY_PLUGIN_ROOT").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return path;
        }
        if let Ok(current_dir) = std::env::current_dir() {
            return current_dir.join(path);
        }
    }
    repository_root().join("plugins/codexy")
}

#[must_use]
pub fn devtools_plugin_root() -> PathBuf {
    if std::env::var_os("CODEXY_PLUGIN_ROOT").is_some_and(|value| !value.is_empty()) {
        return plugin_root();
    }
    repository_root().join("plugins/codexy-devtools")
}

/// Returns the repository root that contains the packaged plugin.
///
/// # Errors
///
/// Returns an error if a relative `CODEXY_REPO_ROOT` cannot be resolved from
/// the current working directory.
pub fn repo_root() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("CODEXY_REPO_ROOT").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return Ok(path);
        }
        return Ok(std::env::current_dir()?.join(path));
    }
    Ok(repository_root().to_path_buf())
}

#[must_use]
pub fn display_relative(path: &Path) -> String {
    repo_root()
        .ok()
        .and_then(|root| path.strip_prefix(root).ok().map(Path::to_path_buf))
        .map_or_else(
            || path.display().to_string(),
            |relative| relative.display().to_string(),
        )
}
