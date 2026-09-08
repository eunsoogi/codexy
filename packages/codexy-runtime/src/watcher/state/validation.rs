use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::mcp::CancellationToken;

use super::MAX_TARGETS;
use super::model::{EVENT_KINDS, Session};
use crate::watcher::io::{canonical_text, token_hash};

const IDENTITY_FIELDS: &[&str] = &["id", "threadId", "taskId", "hostId", "kind", "name"];

pub(super) fn authorize(
    session: &Session,
    token: &str,
    watcher_only: bool,
) -> Result<&'static str> {
    if token.is_empty() || token.len() > 128 {
        bail!("watcher capability is invalid");
    }
    let hash = token_hash(token);
    if hash == session.watcher_token_hash {
        return Ok("watcher");
    }
    if !watcher_only && hash == session.parent_token_hash {
        return Ok("parent");
    }
    bail!("watcher capability is not authorized")
}

pub(super) fn authorize_parent(session: &Session, token: &str) -> Result<()> {
    if token.is_empty() || token.len() > 128 {
        bail!("watcher capability is invalid");
    }
    if token_hash(token) == session.parent_token_hash {
        return Ok(());
    }
    bail!("watcher parent capability is not authorized")
}

pub(super) fn ensure_live(session: &Session, now_ms: u64) -> Result<()> {
    if session.status != "active" {
        bail!("watcher session is {}", session.status);
    }
    if now_ms >= session.expires_at_ms {
        bail!("watcher session has expired");
    }
    Ok(())
}

pub(super) fn same_open(
    session: &Session,
    parent: &Value,
    watcher: &Value,
    targets: &[Value],
) -> bool {
    session.parent == *parent
        && session.watcher == *watcher
        && session.targets.len() == targets.len()
        && session
            .targets
            .iter()
            .zip(targets)
            .all(|(left, right)| canonical_text(left).ok() == canonical_text(right).ok())
}

pub(super) fn validate_identity(value: &Value, label: &str) -> Result<()> {
    if value.as_str().is_some_and(str::is_empty) {
        bail!("watcher {label} must not be empty");
    }
    if let Some(object) = value.as_object() {
        if object
            .keys()
            .any(|key| !IDENTITY_FIELDS.contains(&key.as_str()))
        {
            bail!("watcher {label} contains an unsupported identity field");
        }
        if !object.contains_key("id") && !object.contains_key("threadId") {
            bail!("watcher {label} must contain id or threadId");
        }
        for (key, item) in object {
            let text = item
                .as_str()
                .with_context(|| format!("watcher {label}.{key} must be a string"))?;
            validate_text(text, &format!("{label}.{key}"), 128)?;
        }
        return Ok(());
    }
    if !value.is_string() {
        bail!("watcher {label} must be a string or identity object");
    }
    validate_text(value.as_str().unwrap_or_default(), label, 128)?;
    Ok(())
}

pub(super) fn validate_targets(targets: &[Value]) -> Result<()> {
    if targets.is_empty() || targets.len() > MAX_TARGETS {
        bail!("watcher targets must contain between 1 and {MAX_TARGETS} items");
    }
    for target in targets {
        validate_identity(target, "target")?;
    }
    Ok(())
}

pub(super) fn validate_target(session: &Session, target: &Value) -> Result<()> {
    validate_identity(target, "target")?;
    let key = canonical_text(target)?;
    if !session
        .targets
        .iter()
        .any(|item| canonical_text(item).is_ok_and(|candidate| candidate == key))
    {
        bail!("watcher report target is not assigned to this session");
    }
    Ok(())
}

pub(super) fn validate_event_kind(kind: &str) -> Result<()> {
    if !EVENT_KINDS.contains(&kind) {
        bail!("watcher event kind is not supported: {kind}");
    }
    Ok(())
}

pub(super) fn validate_watcher_state(state: &str) -> Result<()> {
    if ![
        "starting",
        "running",
        "idle",
        "error",
        "stopped",
        "unavailable",
    ]
    .contains(&state)
    {
        bail!("watcher state is not supported: {state}");
    }
    Ok(())
}

pub(super) fn validate_text(value: &str, label: &str, limit: usize) -> Result<()> {
    if value.len() > limit || value.chars().any(char::is_control) {
        bail!("watcher {label} is invalid or exceeds its size limit");
    }
    Ok(())
}

pub(super) fn validate_evidence(evidence: &[String]) -> Result<()> {
    if evidence.len() > 16 {
        bail!("watcher evidence may contain at most 16 locators");
    }
    evidence
        .iter()
        .try_for_each(|item| validate_text(item, "evidence locator", 512))
}

pub(super) fn check_cancelled(cancellation: Option<&CancellationToken>) -> Result<()> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        bail!("watcher wait was cancelled");
    }
    Ok(())
}
