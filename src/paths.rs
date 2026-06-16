use std::path::{Path, PathBuf};

use anyhow::Result;

#[must_use]
pub fn plugin_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
}

/// Returns the repository root that contains the packaged plugin.
///
/// # Errors
///
/// Returns an error if the compile-time plugin root cannot be walked back to
/// the repository root.
pub fn repo_root() -> Result<PathBuf> {
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
