use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::mcp::{CancellationToken, text_result};

use super::state::{MAX_REPORTS, MAX_TTL_SECONDS, MAX_WAIT_MS, Store};

#[must_use]
pub fn call_tool(name: &str, args: &Value) -> Result<Value> {
    call_tool_inner(name, args, None)
}

pub fn call_tool_with_cancellation(
    name: &str,
    args: &Value,
    cancellation: &CancellationToken,
) -> Result<Value> {
    call_tool_inner(name, args, Some(cancellation))
}

fn call_tool_inner(
    name: &str,
    args: &Value,
    cancellation: Option<&CancellationToken>,
) -> Result<Value> {
    let args = args
        .as_object()
        .context("watcher tool arguments must be an object")?;
    let store = Store::new()?;
    let payload = match name {
        "watcher_open" => {
            ensure_keys(
                args,
                &[
                    "assignmentId",
                    "parent",
                    "parentId",
                    "watcher",
                    "watcherId",
                    "targets",
                    "ttlSeconds",
                ],
            )?;
            let assignment = required_string(args.get("assignmentId"), "assignmentId")?;
            let parent = identity(args, "parent", "parentId")?;
            let watcher = identity(args, "watcher", "watcherId")?;
            let targets = args
                .get("targets")
                .and_then(Value::as_array)
                .cloned()
                .context("watcher targets must be an array")?;
            let ttl =
                optional_u64(args.get("ttlSeconds"), "ttlSeconds")?.unwrap_or(MAX_TTL_SECONDS);
            if ttl == 0 || ttl > MAX_TTL_SECONDS {
                bail!("watcher ttlSeconds must be between 1 and {MAX_TTL_SECONDS}");
            }
            store.open(assignment, parent, watcher, targets, ttl)?
        }
        "watcher_report" => {
            ensure_keys(
                args,
                &[
                    "sessionId",
                    "watcherToken",
                    "target",
                    "event",
                    "eventId",
                    "kind",
                    "summary",
                    "evidence",
                    "observedAtMs",
                    "watcherState",
                    "lastError",
                ],
            )?;
            let event = args.get("event").and_then(Value::as_object);
            if args.get("event").is_some() {
                let event = event.context("watcher event must be an object")?;
                ensure_keys(
                    event,
                    &[
                        "target",
                        "eventId",
                        "kind",
                        "summary",
                        "evidence",
                        "observedAtMs",
                        "watcherState",
                        "lastError",
                    ],
                )?;
                for key in [
                    "target",
                    "eventId",
                    "kind",
                    "summary",
                    "evidence",
                    "observedAtMs",
                    "watcherState",
                    "lastError",
                ] {
                    if event.contains_key(key) && args.contains_key(key) {
                        bail!("watcher report field {key} must be provided only once");
                    }
                }
            }
            let session = required_string(args.get("sessionId"), "sessionId")?;
            let token = required_string(args.get("watcherToken"), "watcherToken")?;
            let target = event_field(args, event, "target")
                .cloned()
                .context("watcher target is required")?;
            let kind = optional_event_string(args, event, "kind")?;
            let summary = optional_event_string(args, event, "summary")?;
            let evidence = event_field(args, event, "evidence")
                .map(|value| serde_json::from_value::<Vec<String>>(value.clone()))
                .transpose()
                .context("watcher evidence must be an array of strings")?
                .unwrap_or_default();
            let event_id = optional_event_string(args, event, "eventId")?;
            let observed = optional_event_u64(args, event, "observedAtMs")?;
            let state = optional_event_string(args, event, "watcherState")?;
            let error = optional_event_string(args, event, "lastError")?;
            store.report(
                session.as_str(),
                token.as_str(),
                target,
                kind,
                summary,
                evidence,
                event_id,
                observed,
                state,
                error,
            )?
        }
        "wait_watcher" => {
            ensure_keys(
                args,
                &[
                    "sessionId",
                    "parentToken",
                    "cursor",
                    "maxReports",
                    "timeoutMs",
                ],
            )?;
            let session = required_string(args.get("sessionId"), "sessionId")?;
            let token = required_string(args.get("parentToken"), "parentToken")?;
            let cursor = optional_u64(args.get("cursor"), "cursor")?.unwrap_or(0);
            let max_reports =
                optional_u64(args.get("maxReports"), "maxReports")?.unwrap_or(1) as usize;
            let timeout = optional_u64(args.get("timeoutMs"), "timeoutMs")?.unwrap_or(MAX_WAIT_MS);
            if max_reports == 0 || max_reports > MAX_REPORTS {
                bail!("watcher maxReports must be between 1 and {MAX_REPORTS}");
            }
            if timeout > MAX_WAIT_MS {
                bail!("watcher timeoutMs must be at most {MAX_WAIT_MS}");
            }
            store.wait_with_cancellation(
                session.as_str(),
                token.as_str(),
                cursor,
                max_reports,
                timeout,
                cancellation,
            )?
        }
        "watcher_health" => {
            ensure_keys(args, &["sessionId", "token", "parentToken", "watcherToken"])?;
            let session = required_string(args.get("sessionId"), "sessionId")?;
            let token = args
                .get("token")
                .or_else(|| args.get("parentToken"))
                .or_else(|| args.get("watcherToken"));
            let token = required_string(token, "token")?;
            store.health(session.as_str(), token.as_str())?
        }
        "watcher_cancel" => {
            ensure_keys(args, &["sessionId", "parentToken"])?;
            let session = required_string(args.get("sessionId"), "sessionId")?;
            let token = required_string(args.get("parentToken"), "parentToken")?;
            store.cancel(session.as_str(), token.as_str())?
        }
        _ => bail!("unknown watcher tool: {name}"),
    };
    Ok(text_result(&serde_json::to_string(&payload)?))
}

fn identity(args: &serde_json::Map<String, Value>, primary: &str, legacy: &str) -> Result<Value> {
    if args.contains_key(primary) && args.contains_key(legacy) {
        bail!("watcher identities must use only one of {primary} or {legacy}");
    }
    args.get(primary)
        .or_else(|| args.get(legacy).map(|value| value))
        .cloned()
        .context(format!("watcher {primary} identity is required"))
}

fn ensure_keys(args: &serde_json::Map<String, Value>, allowed: &[&str]) -> Result<()> {
    if let Some(key) = args.keys().find(|key| !allowed.contains(&key.as_str())) {
        bail!("watcher argument is not supported: {key}");
    }
    Ok(())
}

fn required_string(value: Option<&Value>, label: &str) -> Result<String> {
    let text = value
        .and_then(Value::as_str)
        .context(format!("watcher {label} must be a string"))?;
    if text.is_empty() || text.len() > 128 || text.chars().any(char::is_control) {
        bail!("watcher {label} is invalid");
    }
    Ok(text.to_owned())
}

fn optional_u64(value: Option<&Value>, label: &str) -> Result<Option<u64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let number = value
        .as_u64()
        .context(format!("watcher {label} must be an integer"))?;
    Ok(Some(number))
}
fn event_field<'a>(
    args: &'a serde_json::Map<String, Value>,
    event: Option<&'a serde_json::Map<String, Value>>,
    key: &str,
) -> Option<&'a Value> {
    event
        .and_then(|object| object.get(key))
        .or_else(|| args.get(key))
}
fn optional_event_string(
    args: &serde_json::Map<String, Value>,
    event: Option<&serde_json::Map<String, Value>>,
    key: &str,
) -> Result<Option<String>> {
    event_field(args, event, key)
        .map(|value| required_string(Some(value), key))
        .transpose()
}

fn optional_event_u64(
    args: &serde_json::Map<String, Value>,
    event: Option<&serde_json::Map<String, Value>>,
    key: &str,
) -> Result<Option<u64>> {
    optional_u64(event_field(args, event, key), key)
}
