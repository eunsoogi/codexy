mod receipt;

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;
use tempfile::NamedTempFile;

use super::wrappers::{self, WrapperUpdate};

#[derive(Debug)]
struct Update {
    path: PathBuf,
    bytes: Vec<u8>,
}

/// Validates a candidate receipt and atomically stages its activation updates.
/// No publication, commit, branch, or pull request action is performed here.
pub fn activate(repo_root: &Path, bootstrap_version: &str, receipt_path: &Path) -> Result<usize> {
    let updates = prepare(repo_root, bootstrap_version, receipt_path)?;
    apply_with(&updates, write_staged)?;
    Ok(updates.len())
}

fn prepare(repo_root: &Path, bootstrap_version: &str, receipt_path: &Path) -> Result<Vec<Update>> {
    if !repo_root.is_absolute() {
        bail!("repo root must be absolute: {}", repo_root.display());
    }
    if bootstrap_version != super::bootstrap::VERSION {
        bail!(
            "bootstrap version must be public pin {}",
            super::bootstrap::VERSION
        );
    }
    let receipt = read_json(receipt_path, "candidate receipt")?;
    let release = receipt::release_from_receipt(&receipt)?;
    let mut updates = vec![Update {
        path: repo_root.join("plugins/codexy/runtime-release.json"),
        bytes: format!("{}\n", serde_json::to_string_pretty(&release)?).into_bytes(),
    }];
    updates.extend(wrapper_updates(repo_root, bootstrap_version)?);
    Ok(updates)
}

fn apply_with<F>(updates: &[Update], apply: F) -> Result<()>
where
    F: FnOnce(&[Update]) -> Result<()>,
{
    apply(updates)
}

fn wrapper_updates(root: &Path, version: &str) -> Result<Vec<Update>> {
    wrappers::prepare_pin_updates(root, version)?
        .into_iter()
        .map(wrapper_update)
        .collect()
}

fn wrapper_update(update: WrapperUpdate) -> Result<Update> {
    Ok(Update {
        path: update.path,
        bytes: update.bytes,
    })
}

fn write_staged(updates: &[Update]) -> Result<()> {
    let staged = updates.iter().map(stage).collect::<Result<Vec<_>>>()?;
    for (target, temporary) in staged {
        temporary
            .persist(&target)
            .map_err(|error| anyhow::anyhow!("replacing {}: {}", target.display(), error.error))?;
    }
    Ok(())
}

fn stage(update: &Update) -> Result<(PathBuf, NamedTempFile)> {
    let parent = update
        .path
        .parent()
        .context("activation target has no parent")?;
    let mut temporary = NamedTempFile::new_in(parent)
        .with_context(|| format!("staging {}", update.path.display()))?;
    temporary.write_all(&update.bytes)?;
    temporary.as_file().sync_all()?;
    fs::set_permissions(temporary.path(), fs::metadata(&update.path)?.permissions())?;
    Ok((update.path.clone(), temporary))
}

fn read_json(path: &Path, label: &str) -> Result<Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {label}: {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("invalid {label} JSON: {}", path.display()))
}

#[cfg(test)]
#[path = "activation/tests.rs"]
mod tests;
