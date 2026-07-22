use std::path::PathBuf;

use anyhow::Result;

use crate::paths::repo_root;

pub(super) fn paths() -> Result<Vec<PathBuf>> {
    let path = repo_root()?.join("package.json");
    Ok(path.exists().then_some(path).into_iter().collect())
}
