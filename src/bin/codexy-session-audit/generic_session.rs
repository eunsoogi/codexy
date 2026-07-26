use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use super::{Report, SessionReport, audit_math, is_safe_id};

const MAX_METADATA_LINE_BYTES: usize = 256 * 1024;

#[derive(Debug)]
struct Event {
    event_id: String,
    session_id: String,
    cumulative_tokens: u64,
    size_bytes: u64,
    tool_calls: Vec<(String, u64, u64)>,
}

pub(super) fn audit(input: &str, recent_turns: usize) -> Result<Report> {
    let mut sessions = BTreeMap::<String, SessionReport>::new();
    let mut seen_event_ids = BTreeSet::new();
    let mut only_session_id = None;
    let mut duplicate_events_skipped = 0;
    let mut records_observed = 0;
    for (line_number, line) in input.lines().enumerate() {
        if !line.trim().is_empty() {
            records_observed = audit_math::checked_add(records_observed, 1, "window record count")?;
        }
        let Some(event) = parse_event(line, line_number + 1)? else {
            continue;
        };
        if let Some(first_session_id) = &only_session_id {
            if first_session_id != &event.session_id {
                bail!("session metadata must contain exactly one session");
            }
        } else {
            only_session_id = Some(event.session_id.clone());
        }
        if !seen_event_ids.insert(event.event_id.clone()) {
            duplicate_events_skipped += 1;
            continue;
        }
        let session = sessions
            .entry(event.session_id.clone())
            .or_insert_with(|| SessionReport::new(event.session_id, "reported"));
        session.size_bytes =
            audit_math::checked_add(session.size_bytes, event.size_bytes, "session size")?;
        session.latest_cumulative_tokens = event.cumulative_tokens;
        session.cumulative_tokens.push(event.cumulative_tokens);
        session.window.turn_events =
            audit_math::checked_add(session.window.turn_events, 1, "turn event count")?;
        session.record_event_id(event.event_id);
        for (tool, input_bytes, output_bytes) in event.tool_calls {
            let calls = session.tool_calls.entry(tool.clone()).or_default();
            *calls = audit_math::checked_add(*calls, 1, "tool call count")?;
            let inputs = session.tool_input_bytes.entry(tool.clone()).or_default();
            *inputs = audit_math::checked_add(*inputs, input_bytes, "tool input bytes")?;
            let outputs = session.tool_output_bytes.entry(tool).or_default();
            *outputs = audit_math::checked_add(*outputs, output_bytes, "tool output bytes")?;
        }
    }
    let mut reports = sessions.into_values().collect::<Vec<_>>();
    if reports.is_empty() {
        bail!("session metadata must contain exactly one session");
    }
    for session in &mut reports {
        session.window.records_observed = records_observed;
        session.recent_turn_average_tokens =
            audit_math::recent_average(&session.cumulative_tokens, recent_turns)?;
        session.event_ids.sort();
        session.finalize_tool_families()?;
    }
    Ok(Report {
        session_count: reports.len(),
        duplicate_events_skipped,
        sessions: reports,
    })
}

fn parse_event(line: &str, line_number: usize) -> Result<Option<Event>> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    if line.len() > MAX_METADATA_LINE_BYTES {
        bail!("metadata line {line_number} exceeds {MAX_METADATA_LINE_BYTES} bytes");
    }
    let value: Value = serde_json::from_str(line)
        .with_context(|| format!("invalid JSON on metadata line {line_number}"))?;
    let object = value
        .as_object()
        .with_context(|| format!("metadata line {line_number} must be a JSON object"))?;
    if object.get("event").and_then(Value::as_str) != Some("turn.completed") {
        return Ok(None);
    }
    let session_id = required_id(object, "session_id", line_number)?;
    let turn_id = required_id(object, "turn_id", line_number)?;
    let cumulative_tokens = required_u64(object, "cumulative_tokens", line_number)?;
    Ok(Some(Event {
        event_id: format!("turn.completed|{session_id}|{turn_id}"),
        session_id,
        cumulative_tokens,
        size_bytes: u64::try_from(line.len())?,
        tool_calls: parse_tool_calls(object.get("tool_calls"), line_number)?,
    }))
}

fn required_id(
    object: &serde_json::Map<String, Value>,
    key: &str,
    line_number: usize,
) -> Result<String> {
    let value = object.get(key).and_then(Value::as_str).unwrap_or_default();
    if is_safe_id(value) {
        Ok(value.to_owned())
    } else {
        bail!(
            "metadata line {line_number} {key} must contain only ASCII letters, digits, '.', '_', or '-'"
        )
    }
}

fn required_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    line_number: usize,
) -> Result<u64> {
    object.get(key).and_then(Value::as_u64).ok_or_else(|| {
        anyhow::anyhow!("metadata line {line_number} {key} must be an unsigned integer")
    })
}

fn parse_tool_calls(value: Option<&Value>, line_number: usize) -> Result<Vec<(String, u64, u64)>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let calls = value
        .as_array()
        .with_context(|| format!("metadata line {line_number} tool_calls must be an array"))?;
    calls
        .iter()
        .map(|call| {
            let call = call.as_object().with_context(|| {
                format!("metadata line {line_number} tool_calls entries must be objects")
            })?;
            Ok((
                required_id(call, "tool", line_number)?,
                optional_u64(call, "input_bytes", line_number)?,
                required_u64(call, "output_bytes", line_number)?,
            ))
        })
        .collect()
}

fn optional_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    line_number: usize,
) -> Result<u64> {
    match object.get(key) {
        Some(value) => value.as_u64().ok_or_else(|| {
            anyhow::anyhow!("metadata line {line_number} {key} must be an unsigned integer")
        }),
        None => Ok(0),
    }
}
