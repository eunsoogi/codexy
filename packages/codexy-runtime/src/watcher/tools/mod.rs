mod arguments;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::mcp::{CancellationToken, text_result};

use super::state::{DEFAULT_WAIT_MS, MAX_REPORTS, MAX_TTL_SECONDS, MAX_WAIT_MS, Store};
use arguments::{
    ensure_keys, event_field, identity, optional_cursor, optional_event_string, optional_event_u64,
    optional_u64, required_string,
};

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
                event_id.as_deref(),
                observed,
                state,
                error,
            )?
        }
        "watcher_wait" => {
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
            let cursor = optional_cursor(args.get("cursor"))?.unwrap_or(0);
            let max_reports =
                usize::try_from(optional_u64(args.get("maxReports"), "maxReports")?.unwrap_or(1))
                    .context("watcher maxReports is out of range")?;
            let timeout =
                optional_u64(args.get("timeoutMs"), "timeoutMs")?.unwrap_or(DEFAULT_WAIT_MS);
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
