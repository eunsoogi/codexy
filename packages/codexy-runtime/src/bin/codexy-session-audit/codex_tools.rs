use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde_json::Value;

use super::{SessionReport, checked_add, is_safe_id};

pub(super) fn record(
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
        record_call(
            object,
            report,
            seen_calls,
            call_names,
            duplicates,
            line_number,
            item_type,
            call_id,
            call_key,
        )
    } else if is_tool_output(item_type) {
        record_output(
            object,
            report,
            seen_outputs,
            call_names,
            duplicates,
            item_type,
            call_id,
            call_key,
        )
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn record_call(
    object: &serde_json::Map<String, Value>,
    report: &mut SessionReport,
    seen_calls: &mut BTreeSet<String>,
    call_names: &mut BTreeMap<String, String>,
    duplicates: &mut u64,
    line_number: usize,
    item_type: &str,
    _call_id: String,
    call_key: String,
) -> Result<()> {
    let name = nested_id(object, &["payload", "name"], line_number)?;
    if !seen_calls.insert(call_key.clone()) {
        if call_names
            .get(&call_key)
            .is_some_and(|first| first != &name)
        {
            bail!("metadata line {line_number} has conflicting tool names for one call identity");
        }
        *duplicates += 1;
        return Ok(());
    }
    let call_count = report.tool_calls.entry(name.clone()).or_default();
    *call_count = checked_add(*call_count, 1, "tool call count")?;
    let input_bytes = match item_type {
        "function_call" => value_bytes(
            object
                .get("payload")
                .and_then(|payload| payload.get("arguments")),
        )?,
        "custom_tool_call" => value_bytes(
            object
                .get("payload")
                .and_then(|payload| payload.get("input")),
        )?,
        _ => 0,
    };
    let total_input = report.tool_input_bytes.entry(name.clone()).or_default();
    *total_input = checked_add(*total_input, input_bytes, "tool input bytes")?;
    report.record_event_id(call_key.clone());
    call_names.insert(call_key, name);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_output(
    object: &serde_json::Map<String, Value>,
    report: &mut SessionReport,
    seen_outputs: &mut BTreeSet<String>,
    call_names: &BTreeMap<String, String>,
    duplicates: &mut u64,
    item_type: &str,
    call_id: String,
    call_key: String,
) -> Result<()> {
    let Some(name) = call_names.get(&call_key) else {
        return Ok(());
    };
    let output_key = format!("{}|{item_type}|{call_id}", report.session_id);
    if !seen_outputs.insert(output_key.clone()) {
        *duplicates += 1;
        return Ok(());
    }
    let bytes = value_bytes(
        object
            .get("payload")
            .and_then(|payload| payload.get("output")),
    )?;
    let total_bytes = report.tool_output_bytes.entry(name.clone()).or_default();
    *total_bytes = checked_add(*total_bytes, bytes, "tool output bytes")?;
    report.record_event_id(output_key);
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

fn value_bytes(value: Option<&Value>) -> Result<u64> {
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
