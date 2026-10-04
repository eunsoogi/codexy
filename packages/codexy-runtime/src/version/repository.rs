//! Resolves version-managed paths relative to the active repository root.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::repo_path;

pub(super) fn runtime_package_path(root: &Path, relative: &str) -> PathBuf {
    root.join("packages/codexy-runtime").join(relative)
}

pub(super) fn package_manifests() -> Result<Vec<PathBuf>> {
    let path = repo_path("package.json")?;
    // Repositories without the optional Node package projection remain valid Rust-only checkouts.
    Ok(if path.exists() {
        vec![path]
    } else {
        Vec::new()
    })
}
