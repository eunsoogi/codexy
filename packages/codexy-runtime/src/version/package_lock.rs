use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde_json::Value;

use super::repo_path;

const PACKAGE_LOCK: &str = "package-lock.json";

pub(super) fn package_locks() -> Result<Vec<PathBuf>> {
    let path = repo_path(PACKAGE_LOCK)?;
    Ok(if path.exists() {
        vec![path]
    } else {
        Vec::new()
    })
}

pub(super) fn root_package_lock<'a>(lock: &'a Value, path: &Path) -> Result<&'a Value> {
    lock.get("packages")
        .and_then(Value::as_object)
        .and_then(|packages| packages.get(""))
        .with_context(|| format!("{} must contain a root package", path.display()))
}

pub(super) fn root_package_lock_mut<'a>(lock: &'a mut Value, path: &Path) -> Result<&'a mut Value> {
    lock.get_mut("packages")
        .and_then(Value::as_object_mut)
        .and_then(|packages| packages.get_mut(""))
        .with_context(|| format!("{} must contain a root package", path.display()))
}
