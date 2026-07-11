use std::path::{Path, PathBuf};

use anyhow::Result;

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
    Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
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
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf())
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
