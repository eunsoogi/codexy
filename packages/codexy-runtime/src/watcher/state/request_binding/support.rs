use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use super::{ARMED_TTL_MS, Binding, DIRECTORY, MAX_WAIT_MS};
use crate::watcher::io::{ensure_dir, read_json, reject_link, safe_id};
use crate::watcher::lock::LockGuard;

pub(super) fn bindings_dir(root: &Path) -> Result<PathBuf> {
    let directory = root.join(DIRECTORY);
    ensure_dir(&directory)?;
    Ok(directory)
}

pub(super) fn binding_path(directory: &Path, nonce: &str) -> Result<PathBuf> {
    valid_nonce(nonce)?;
    Ok(directory.join(format!("{nonce}.json")))
}

pub(super) fn cancellation_path(directory: &Path, nonce: &str) -> Result<PathBuf> {
    valid_nonce(nonce)?;
    Ok(directory.join(format!("{nonce}.cancel")))
}

pub(super) fn live_bindings(root: &Path, directory: &Path, now: u64) -> Result<Vec<Binding>> {
    let mut bindings = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_symlink() {
            reject_link(&path)?;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(binding) = read_json::<Binding>(&path, "request binding") else {
            continue;
        };
        if validate(&binding).is_err() {
            continue;
        }
        if binding.status == "active" && active_wait(root, &binding)? {
            bindings.push(binding);
            continue;
        }
        if binding.status == "active" || now >= binding.expires_at_ms {
            remove_owned(&path);
            if let Ok(cancel) = cancellation_path(directory, &binding.nonce) {
                remove_owned(&cancel);
            }
            continue;
        }
        bindings.push(binding);
    }
    Ok(bindings)
}

fn active_wait(root: &Path, binding: &Binding) -> Result<bool> {
    let session_dir = root.join(&binding.watcher_session_id);
    reject_link(&session_dir)?;
    if !session_dir.is_dir() {
        return Ok(false);
    }
    Ok(LockGuard::try_acquire(&session_dir.join("wait.lock"))?.is_none())
}

pub(super) fn validate(binding: &Binding) -> Result<()> {
    if binding.schema != 1 || !matches!(binding.status.as_str(), "armed" | "active") {
        bail!("watcher request binding schema is invalid");
    }
    valid_nonce(&binding.nonce)?;
    for (value, label) in [
        (&binding.main_session_id, "main session id"),
        (&binding.turn_id, "turn id"),
        (&binding.tool_use_id, "tool use id"),
        (&binding.watcher_session_id, "session id"),
    ] {
        text(Some(&Value::String(value.clone())), label, 256)?;
    }
    let max_lifetime = if binding.status == "active" {
        ARMED_TTL_MS.saturating_add(MAX_WAIT_MS)
    } else {
        ARMED_TTL_MS
    };
    if binding.parent_token_hash.len() != 64
        || !binding
            .parent_token_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || binding.expires_at_ms <= binding.created_at_ms
        || binding.expires_at_ms - binding.created_at_ms > max_lifetime
    {
        bail!("watcher request binding fields are invalid");
    }
    Ok(())
}

pub(super) fn valid_nonce(nonce: &str) -> Result<()> {
    safe_id(nonce, "request binding")?;
    if nonce.len() != 64 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("watcher request binding nonce is invalid");
    }
    Ok(())
}

pub(super) fn text(value: Option<&Value>, label: &str, limit: usize) -> Result<String> {
    let value = value
        .and_then(Value::as_str)
        .with_context(|| format!("watcher {label} must be a string"))?;
    if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        bail!("watcher {label} is invalid");
    }
    Ok(value.to_owned())
}

pub(super) fn remove_owned(path: &Path) {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file()) {
        let _ = fs::remove_file(path);
    }
}
