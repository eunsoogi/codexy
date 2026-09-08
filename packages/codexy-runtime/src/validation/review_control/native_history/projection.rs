#[path = "projection/snapshot.rs"]
mod snapshot;
#[path = "projection/values.rs"]
mod values;

use serde_json::{Value, json};

use super::RECEIPT_SCHEMA;
use super::source::Captured;

pub(crate) fn project(captured: Captured) -> Result<Value, String> {
    let events = captured
        .events
        .iter()
        .map(values::event_value)
        .collect::<Result<Vec<_>, _>>()?;
    let history = captured
        .events
        .iter()
        .map(values::history_value)
        .collect::<Vec<_>>();
    let full = captured
        .events
        .iter()
        .filter(|event| event.kind == "full")
        .count();
    let delta = captured
        .events
        .iter()
        .filter(|event| event.kind == "delta")
        .count();
    let required_head = captured
        .events
        .iter()
        .filter(|event| event.kind == "required_current_head")
        .count();
    let mut receipt = json!({
        "schema": RECEIPT_SCHEMA,
        "target": captured.target,
        "source": {
            "owner": {"thread_id": captured.owner_thread, "pages": captured.owner_pages},
            "reviewer": {"thread_id": captured.reviewer_thread, "pages": captured.reviewer_pages}
        },
        "owner": {
            "invocation": values::invocation_value(&captured.invocation),
            "helpers": captured.helpers
        },
        "events": events,
        "history_projection": {
            "terminal_review_history": history,
            "full_review_count": full,
            "delta_review_count": delta,
            "required_current_head_count": required_head,
            "terminal_review_count": captured.events.len()
        },
        "admission": {
            "history": "recovered",
            "current_pr": "unbound",
            "authentication": "not_attested",
            "temporal": "unbound",
            "result": "not_admitted"
        }
    });
    if let Some(capture) = captured.owner_capture {
        receipt["source"]["owner"]["capture"] = capture;
    }
    if let Some(capture) = captured.reviewer_capture {
        receipt["source"]["reviewer"]["capture"] = capture;
    }
    if let Some(snapshot) = captured.current_pr_snapshot {
        receipt["current_pr_snapshot"] = snapshot;
    }
    Ok(receipt)
}

pub(crate) fn bind(receipt: &Value, current_snapshot: &Value) -> Result<Value, String> {
    let receipt_map = object(receipt, "native-history receipt")?;
    if text(receipt_map, "schema")? != Some(RECEIPT_SCHEMA) {
        return Err("native-history receipt has an unsupported schema".into());
    }
    let target = object(
        receipt_map
            .get("target")
            .ok_or("native-history receipt must contain target")?,
        "receipt target",
    )?;
    let snapshot_map = object(current_snapshot, "current PR snapshot")?;
    snapshot::validate(snapshot_map, target)?;
    let temporal = snapshot::temporal(receipt_map, snapshot_map)?;
    let mut bound = receipt.clone();
    let bound_map = bound
        .as_object_mut()
        .ok_or("native-history receipt must be an object")?;
    bound_map.insert("current_pr_snapshot".into(), current_snapshot.clone());
    let admission = bound_map
        .get_mut("admission")
        .and_then(Value::as_object_mut)
        .ok_or("native-history receipt must contain admission")?;
    admission.insert("current_pr".into(), Value::String("bound".into()));
    admission.insert("temporal".into(), Value::String(temporal.clone()));
    admission.insert(
        "authentication".into(),
        Value::String("not_attested".into()),
    );
    admission.insert("result".into(), Value::String("not_admitted".into()));
    bound_map.insert(
        "binding".into(),
        json!({
            "current_head": snapshot_map.get("headRefOid"),
            "temporal": temporal,
            "existing_history_preserved": snapshot_map.contains_key("reviewControl")
        }),
    );
    Ok(bound)
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

fn text<'a>(map: &'a serde_json::Map<String, Value>, key: &str) -> Result<Option<&'a str>, String> {
    match map.get(key) {
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value)),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(format!("{key} must be a non-empty string")),
    }
}
