use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::model::Session;
use super::validation::{authorize_parent, ensure_live};
use crate::watcher::io::{
    now_ms, random_hex, read_json, reject_link, state_root, token_hash, write_json,
};
use crate::watcher::lock::LockGuard;

mod support;

use support::{
    binding_path, bindings_dir, cancellation_path, live_bindings, remove_owned, text, valid_nonce,
    validate,
};

const DIRECTORY: &str = ".request-bindings";
const LOCK: &str = ".request-bindings.lock";
const MAX_BINDINGS: usize = 128;
const MAX_TTL_MS: u64 = 60_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    schema: u8,
    nonce: String,
    main_session_id: String,
    turn_id: String,
    tool_use_id: String,
    watcher_session_id: String,
    parent_token_hash: String,
    created_at_ms: u64,
    expires_at_ms: u64,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cancellation {
    schema: u8,
    nonce: String,
    cancelled_at_ms: u64,
}

#[derive(Debug)]
pub(super) struct Guard {
    path: PathBuf,
    cancellation_path: PathBuf,
}

#[allow(unreachable_pub)]
pub fn prepare_request_binding(payload: &Value) -> Result<String> {
    let root = state_root()?;
    let main_session_id = text(payload.get("session_id"), "session_id", 256)?;
    let turn_id = text(payload.get("turn_id"), "turn_id", 256)?;
    let tool_use_id = text(payload.get("tool_use_id"), "tool_use_id", 256)?;
    let tool_name = text(payload.get("tool_name"), "tool_name", 256)?;
    if tool_name != "watcher_wait" && !tool_name.ends_with("__watcher_wait") {
        bail!("watcher request binding is limited to watcher_wait");
    }
    let input = payload
        .get("tool_input")
        .and_then(Value::as_object)
        .context("watcher tool_input must be an object")?;
    let session_id = text(input.get("sessionId"), "sessionId", 128)?;
    let parent_token = text(input.get("parentToken"), "parentToken", 128)?;
    let session_path = root.join(&session_id);
    reject_link(&session_path)?;
    let session: Session = read_json(&session_path.join("session.json"), "session")?;
    authorize_parent(&session, &parent_token)?;
    ensure_live(&session, now_ms())?;

    let directory = bindings_dir(&root)?;
    let _lock = LockGuard::acquire(&root.join(LOCK), 500)?;
    let now = now_ms();
    let bindings = live_bindings(&directory, now)?;
    if bindings
        .iter()
        .any(|binding| binding.main_session_id == main_session_id && binding.turn_id == turn_id)
    {
        bail!("watcher request binding already exists for this turn");
    }
    if bindings.len() >= MAX_BINDINGS {
        bail!("watcher request binding storage is full");
    }
    let nonce = random_hex(32)?;
    let binding = Binding {
        schema: 1,
        nonce: nonce.clone(),
        main_session_id,
        turn_id,
        tool_use_id,
        watcher_session_id: session_id,
        parent_token_hash: token_hash(&parent_token),
        created_at_ms: now,
        expires_at_ms: now.saturating_add(MAX_TTL_MS),
        status: "armed".to_owned(),
    };
    write_json(
        &binding_path(&directory, &nonce)?,
        &serde_json::to_value(binding)?,
    )?;
    Ok(nonce)
}

#[allow(unreachable_pub)]
pub fn interrupt_request_binding(payload: &Value) -> Result<bool> {
    let root = state_root()?;
    let main_session_id = text(payload.get("session_id"), "session_id", 256)?;
    let turn_id = text(payload.get("turn_id"), "turn_id", 256)?;
    let directory = bindings_dir(&root)?;
    let _lock = LockGuard::acquire(&root.join(LOCK), 500)?;
    let matches = live_bindings(&directory, now_ms())?
        .into_iter()
        .filter(|binding| {
            binding.status == "active"
                && binding.main_session_id == main_session_id
                && binding.turn_id == turn_id
        })
        .collect::<Vec<_>>();
    let Some(binding) = matches.first().filter(|_| matches.len() == 1) else {
        return Ok(false);
    };
    write_json(
        &cancellation_path(&directory, &binding.nonce)?,
        &json!({
            "schema": 1,
            "nonce": binding.nonce,
            "cancelledAtMs": now_ms(),
        }),
    )?;
    Ok(true)
}

pub(super) fn claim(
    root: &Path,
    nonce: &str,
    session_id: &str,
    parent_token: &str,
) -> Result<Guard> {
    valid_nonce(nonce)?;
    let directory = bindings_dir(root)?;
    let _lock = LockGuard::acquire(&root.join(LOCK), 2_000)?;
    let path = binding_path(&directory, nonce)?;
    let mut binding: Binding = read_json(&path, "request binding")?;
    validate(&binding)?;
    if binding.status != "armed" {
        bail!("watcher request binding is not armed");
    }
    if binding.watcher_session_id != session_id {
        bail!("watcher request binding is bound to another session");
    }
    if token_hash(parent_token) != binding.parent_token_hash {
        bail!("watcher request binding capability is not authorized");
    }
    if now_ms() >= binding.expires_at_ms {
        bail!("watcher request binding has expired");
    }
    "active".clone_into(&mut binding.status);
    write_json(&path, &serde_json::to_value(binding)?)?;
    Ok(Guard {
        cancellation_path: cancellation_path(&directory, nonce)?,
        path,
    })
}

pub(super) fn cancelled(root: &Path, nonce: &str) -> Result<bool> {
    valid_nonce(nonce)?;
    let path = cancellation_path(&bindings_dir(root)?, nonce)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_file() {
        bail!("watcher request cancellation must be a regular file");
    }
    let marker: Cancellation = read_json(&path, "request cancellation")?;
    Ok(marker.schema == 1 && marker.nonce == nonce && marker.cancelled_at_ms > 0)
}

impl Drop for Guard {
    fn drop(&mut self) {
        remove_owned(&self.path);
        remove_owned(&self.cancellation_path);
    }
}
