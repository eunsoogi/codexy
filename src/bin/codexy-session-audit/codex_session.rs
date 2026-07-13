use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde_json::Value;

use super::{Report, SessionReport, audit_math::checked_add, is_safe_id};

const MAX_METADATA_LINE_BYTES: usize = 256 * 1024;

pub(super) fn is_codex_session(input: &str) -> bool {
    input.lines().any(|line| {
        serde_json::from_str::<Value>(line)
            .ok()
            .and_then(|value| value.get("type").and_then(Value::as_str).map(str::to_owned))
            .as_deref()
            == Some("session_meta")
    })
}

pub(super) fn audit(input: &str, recent_turns: usize) -> Result<Report> {
    let mut session_id = None;
    let mut session = None;
    let mut seen_events = BTreeSet::new();
    let mut seen_calls = BTreeSet::new();
    let mut seen_outputs = BTreeSet::new();
    let mut call_names = BTreeMap::new();
    let mut per_turn_tokens = Vec::new();
    let mut duplicates = 0;
    for (line_number, line) in input.lines().enumerate() {
        if line.len() > MAX_METADATA_LINE_BYTES {
            bail!(
                "metadata line {} exceeds {MAX_METADATA_LINE_BYTES} bytes",
                line_number + 1
            );
        }
        let value: Value = serde_json::from_str(line).map_err(|error| {
            anyhow::anyhow!("invalid JSON on metadata line {}: {error}", line_number + 1)
        })?;
        let object = value.as_object().ok_or_else(|| {
            anyhow::anyhow!("metadata line {} must be a JSON object", line_number + 1)
        })?;
        let kind = object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if kind == "session_meta" {
            let id = session_meta_id(object, line_number)?;
            if session_id.is_some() {
                bail!("Codex session metadata must contain exactly one session_meta");
            }
            session_id = Some(id.clone());
            session = Some(SessionReport::new(id));
            continue;
        }
        let Some(report) = session.as_mut() else {
            continue;
        };
        if kind == "event_msg" && nested_str(object, &["payload", "type"]) == Some("token_count") {
            let Some(info) = object
                .get("payload")
                .and_then(|payload| payload.get("info"))
            else {
                continue;
            };
            if info.is_null() {
                continue;
            }
            let tokens = nested_u64(
                object,
                &["payload", "info", "total_token_usage", "total_tokens"],
                line_number,
            )?;
            let event_id = format!(
                "token_count|{}|{tokens}",
                session_id.as_deref().unwrap_or_default()
            );
            if seen_events.insert(event_id.clone()) {
                let last_tokens = nested_u64(
                    object,
                    &["payload", "info", "last_token_usage", "total_tokens"],
                    line_number,
                )?;
                report.latest_cumulative_tokens = tokens;
                report.cumulative_tokens.push(tokens);
                per_turn_tokens.push(last_tokens);
                report.record_event_id(event_id);
            } else {
                duplicates += 1;
            }
        } else if kind == "response_item" {
            record_tool_metadata(
                object,
                report,
                &mut seen_calls,
                &mut seen_outputs,
                &mut call_names,
                &mut duplicates,
                line_number,
            )?;
        }
    }
    let Some(mut session) = session else {
        bail!("Codex session metadata is missing session_meta");
    };
    session.size_bytes = u64::try_from(input.len())?;
    session.recent_turn_average_tokens = recent_direct_average(&per_turn_tokens, recent_turns)?;
    session.event_ids.sort();
    Ok(Report {
        session_count: 1,
        duplicate_events_skipped: duplicates,
        sessions: vec![session],
    })
}
fn session_meta_id(object: &serde_json::Map<String, Value>, line_number: usize) -> Result<String> {
    let value = nested_str(object, &["payload", "id"])
        .or_else(|| nested_str(object, &["payload", "session_id"]))
        .unwrap_or_default();
    if is_safe_id(value) {
        Ok(value.to_owned())
    } else {
        bail!("metadata line {line_number} payload.id must be a safe id")
    }
}
fn recent_direct_average(tokens: &[u64], recent_turns: usize) -> Result<u64> {
    let recent = &tokens[tokens.len().saturating_sub(recent_turns)..];
    let count = u64::try_from(recent.len()).unwrap_or(0);
    if count == 0 {
        return Ok(0);
    }
    let total = recent.iter().try_fold(0, |total, tokens| {
        checked_add(total, *tokens, "recent token total")
    })?;
    Ok(total / count)
}
fn record_tool_metadata(
    object: &serde_json::Map<String, Value>,
    report: &mut SessionReport,
    seen_calls: &mut BTreeSet<String>,
    seen_outputs: &mut BTreeSet<String>,
    call_names: &mut BTreeMap<String, String>,
    duplicates: &mut u64,
    line_number: usize,
) -> Result<()> {
    let item_type = nested_str(object, &["payload", "type"]).unwrap_or_default();
    let Some(_) = nested_str(object, &["payload", "call_id"]) else {
        return Ok(());
    };
    let call_id = nested_id(object, &["payload", "call_id"], line_number)?;
    let Some(call_key) = call_key(report.session_id.as_str(), item_type, &call_id) else {
        return Ok(());
    };
    if is_tool_call(item_type) {
        let name = nested_id(object, &["payload", "name"], line_number)?;
        if seen_calls.insert(call_key.clone()) {
            let call_count = report.tool_calls.entry(name.clone()).or_default();
            *call_count = checked_add(*call_count, 1, "tool call count")?;
            report.record_event_id(call_key.clone());
            call_names.insert(call_key, name);
        } else {
            if call_names
                .get(&call_key)
                .is_some_and(|first| first != &name)
            {
                bail!(
                    "metadata line {line_number} has conflicting tool names for one call identity"
                );
            }
            *duplicates += 1;
        }
    } else if is_tool_output(item_type) {
        let Some(name) = call_names.get(&call_key) else {
            return Ok(());
        };
        let output_key = format!("{}|{item_type}|{call_id}", report.session_id);
        if !seen_outputs.insert(output_key.clone()) {
            *duplicates += 1;
            return Ok(());
        }
        let bytes = output_bytes(
            object
                .get("payload")
                .and_then(|payload| payload.get("output")),
        )?;
        let total_bytes = report.tool_output_bytes.entry(name.clone()).or_default();
        *total_bytes = checked_add(*total_bytes, bytes, "tool output bytes")?;
        report.record_event_id(output_key);
    }
    Ok(())
}
fn call_key(session_id: &str, item_type: &str, call_id: &str) -> Option<String> {
    let call_type = item_type.strip_suffix("_output").unwrap_or(item_type);
    is_tool_call(call_type).then(|| format!("{session_id}|{call_type}|{call_id}"))
}
fn is_tool_call(item_type: &str) -> bool {
    matches!(item_type, "function_call" | "custom_tool_call")
}
fn is_tool_output(item_type: &str) -> bool {
    matches!(
        item_type,
        "function_call_output" | "custom_tool_call_output"
    )
}
fn output_bytes(value: Option<&Value>) -> Result<u64> {
    let bytes = match value {
        Some(Value::String(text)) => text.len(),
        Some(value) => serde_json::to_vec(value)?.len(),
        None => 0,
    };
    Ok(u64::try_from(bytes)?)
}
fn nested_str<'a>(object: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    let mut value = object.get(*keys.first()?)?;
    for key in &keys[1..] {
        value = value.get(*key)?;
    }
    value.as_str()
}
fn nested_id(
    object: &serde_json::Map<String, Value>,
    keys: &[&str],
    line_number: usize,
) -> Result<String> {
    let value = nested_str(object, keys).unwrap_or_default();
    if is_safe_id(value) {
        Ok(value.to_owned())
    } else {
        bail!(
            "metadata line {line_number} {} must be a safe id",
            keys.join(".")
        )
    }
}

fn nested_u64(
    object: &serde_json::Map<String, Value>,
    keys: &[&str],
    line_number: usize,
) -> Result<u64> {
    let mut value = object.get(*keys.first().unwrap_or(&""));
    for key in &keys[1..] {
        value = value.and_then(|current| current.get(*key));
    }
    value.and_then(Value::as_u64).ok_or_else(|| {
        anyhow::anyhow!(
            "metadata line {line_number} {} must be an unsigned integer",
            keys.join(".")
        )
    })
}
