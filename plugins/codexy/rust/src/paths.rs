use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

pub fn plugin_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Returns the repository root that contains the packaged plugin.
///
/// # Errors
///
/// Returns an error if the compile-time plugin root cannot be walked back to
/// the repository root.
pub fn repo_root() -> Result<PathBuf> {
    plugin_root()
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("resolving repository root from plugin root")
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
