use std::path::{Path, PathBuf};

use anyhow::Result;

use super::repo_path;

pub(super) fn runtime_package_path(root: &Path, relative: &str) -> PathBuf {
    root.join("packages/codexy-runtime").join(relative)
}

pub(super) fn package_manifests() -> Result<Vec<PathBuf>> {
    let path = repo_path("package.json")?;
    Ok(if path.exists() {
        vec![path]
    } else {
        Vec::new()
    })
}
