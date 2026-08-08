use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

pub(super) const RUNTIME_SUITE: &str = "packages/codexy-runtime/tests/suites/all.rs";

pub(super) fn runtime_path(plugin_root: &Path) -> Result<PathBuf> {
    let parent = plugin_root
        .parent()
        .ok_or_else(|| anyhow!("plugin root must have a repository parent"))?;
    let repository = if parent.file_name().is_some_and(|name| name == "plugins") {
        parent
            .parent()
            .ok_or_else(|| anyhow!("plugin root plugins directory must have a repository parent"))?
    } else {
        parent
    };
    Ok(repository.join(RUNTIME_SUITE))
}
