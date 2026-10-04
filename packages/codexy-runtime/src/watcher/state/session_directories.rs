//! Enumerates session storage without mistaking Watcher-owned auxiliary state for a session.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::request_binding;
use crate::watcher::io::{reject_link, safe_id};

pub(super) fn list(root: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            reject_link(&path)?;
        }
        if !file_type.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("watcher session id is not UTF-8"))?;
        if name == request_binding::DIRECTORY {
            // Active hook bindings share the state root but must never enter session reclamation.
            continue;
        }
        safe_id(&name, "sessionId")?;
        directories.push(path);
    }
    Ok(directories)
}
