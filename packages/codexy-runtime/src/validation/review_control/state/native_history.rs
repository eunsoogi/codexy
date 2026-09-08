use std::path::Path;

use serde_json::{Map, Value, json};

use super::super::{native_history as receipt, snapshot};

pub(super) fn check_predecessor(plugin_root: &Path, state: &Value) -> Result<(), String> {
    let control = state
        .get("reviewControl")
        .and_then(Value::as_object)
        .ok_or("native history predecessor must contain reviewControl")?;
    if !control.contains_key("native_history_recovery") {
        return Err("native history predecessor marker is missing".into());
    }
    check_provenance(state, control)?;
    let bound_snapshot = state
        .get("nativeHistoryRecovery")
        .and_then(Value::as_object)
        .and_then(|receipt| receipt.get("current_pr_snapshot"))
        .ok_or("native history predecessor must carry a bound snapshot")?;
    let mut snapshot = state.clone();
    let snapshot_map = snapshot
        .as_object_mut()
        .ok_or("native history predecessor must be an object")?;
    snapshot_map.remove("reviewControl");
    snapshot_map.remove("nativeHistoryRecovery");
    if snapshot != *bound_snapshot {
        return Err("native history predecessor changed its bound PR snapshot".into());
    }
    super::check_control(
        plugin_root,
        state
            .get("reviewControl")
            .ok_or("native history predecessor must contain reviewControl")?,
    )
}

pub(super) fn check_provenance(state: &Value, control: &Map<String, Value>) -> Result<(), String> {
    let receipt_value = state
        .get("nativeHistoryRecovery")
        .ok_or("native history provenance must carry bound receipt")?;
    let receipt_map = receipt_value
        .as_object()
        .ok_or("native history recovery receipt must be an object")?;
    let bound_snapshot = receipt_map
        .get("current_pr_snapshot")
        .ok_or("native history recovery receipt must carry bound snapshot")?;
    snapshot::check(bound_snapshot, "native history recovery")?;
    if bound_snapshot.get("reviewControl").is_some()
        || bound_snapshot.get("nativeHistoryRecovery").is_some()
    {
        return Err("native history recovery bound snapshot must be a clean PR snapshot".into());
    }
    let rebound = receipt::bind_current_pr_snapshot(receipt_value, bound_snapshot)?;
    if rebound != *receipt_value {
        return Err("native history recovery receipt is not bound to its source snapshot".into());
    }
    let admission = receipt_map
        .get("admission")
        .and_then(Value::as_object)
        .ok_or("native history recovery receipt must carry admission")?;
    if admission.get("current_pr").and_then(Value::as_str) != Some("bound")
        || admission.get("temporal").and_then(Value::as_str) != Some("proved_post_pr")
        || admission.get("authentication").and_then(Value::as_str) != Some("not_attested")
        || admission.get("result").and_then(Value::as_str) != Some("not_admitted")
    {
        return Err("native history recovery receipt has an invalid admission boundary".into());
    }
    let binding = receipt_map
        .get("binding")
        .and_then(Value::as_object)
        .ok_or("native history recovery receipt must carry binding")?;
    if binding.get("current_head") != bound_snapshot.get("headRefOid")
        || binding.get("temporal") != admission.get("temporal")
        || binding.get("existing_history_preserved") != Some(&Value::Bool(false))
    {
        return Err("native history recovery receipt has an invalid binding".into());
    }
    let source = receipt_map
        .get("source")
        .and_then(Value::as_object)
        .ok_or("native history recovery receipt must carry source pages")?;
    let owner_pages = source
        .get("owner")
        .and_then(Value::as_object)
        .and_then(|owner| owner.get("pages"))
        .ok_or("native history recovery receipt must carry owner pages")?;
    let reviewer_pages = source
        .get("reviewer")
        .and_then(Value::as_object)
        .and_then(|reviewer| reviewer.get("pages"))
        .ok_or("native history recovery receipt must carry reviewer pages")?;
    let owner = source_role_request(source, "owner", owner_pages)?;
    let reviewer = source_role_request(source, "reviewer", reviewer_pages)?;
    let source_request = json!({
        "schema": receipt::REQUEST_SCHEMA,
        "target": receipt_map.get("target"),
        "owner": owner,
        "reviewer": reviewer
    });
    let projected = receipt::normalize_native_history(&source_request)?;
    for key in [
        "schema",
        "target",
        "source",
        "owner",
        "events",
        "history_projection",
    ] {
        if receipt_map.get(key) != projected.get(key) {
            return Err(format!(
                "native history recovery receipt does not match preserved source {key}"
            ));
        }
    }
    snapshot::same_pr(bound_snapshot, state)?;
    snapshot::same_issue(bound_snapshot, state)?;
    let current_issue = snapshot::owning_issue_number(state, "current")?;
    if control.get("issue_number").and_then(Value::as_u64) != Some(current_issue) {
        return Err("native history provenance changes the owning issue".into());
    }
    if state.get("reviewProfile") != control.get("profile") {
        return Err("native history provenance changes the review profile".into());
    }
    let receipt_events = receipt_map
        .get("events")
        .and_then(Value::as_array)
        .ok_or("native history recovery receipt must carry events")?;
    if receipt_events.is_empty() || receipt_events.len() > 2 {
        return Err("native history provenance must contain full and optional delta".into());
    }
    for (index, event) in receipt_events.iter().enumerate() {
        let expected = if index == 0 { "full" } else { "delta" };
        if event.get("kind").and_then(Value::as_str) != Some(expected) {
            return Err(
                "native history provenance must be ordered full then optional delta".into(),
            );
        }
    }
    let receipt_ids = receipt_events
        .iter()
        .map(|event| {
            event
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .ok_or("native history provenance event id is missing")
        })
        .collect::<Result<Vec<_>, _>>()?;
    for marker_name in ["native_history_provenance", "native_history_recovery"] {
        if let Some(marker) = control.get(marker_name) {
            let marker = marker
                .as_object()
                .ok_or("native history provenance marker must be an object")?;
            if marker.get("schema").and_then(Value::as_str) != Some(receipt::RECEIPT_SCHEMA)
                || marker.get("temporal").and_then(Value::as_str) != Some("proved_post_pr")
            {
                return Err("native history provenance marker is invalid".into());
            }
            let marker_ids = marker
                .get("event_ids")
                .and_then(Value::as_array)
                .ok_or("native history provenance marker must carry event_ids")?;
            if marker_ids
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()
                != Some(receipt_ids.clone())
            {
                return Err("native history provenance marker does not bind receipt events".into());
            }
        }
    }
    let history = control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .ok_or("native history provenance requires terminal history")?;
    if history.len() < receipt_events.len() {
        return Err("native history terminal history is shorter than its receipt".into());
    }
    for (history_event, receipt_event) in history.iter().zip(receipt_events) {
        for key in ["id", "kind", "reviewed_head", "terminal_result"] {
            if history_event.get(key) != receipt_event.get(key) {
                return Err(format!(
                    "native history terminal history does not bind receipt {key}"
                ));
            }
        }
        if history_event.get("unresolved_findings") != receipt_event.get("findings")
            || history_event.get("reviewer") != receipt_event.get("reviewer")
            || history_event.get("source_reviewer") != receipt_event.get("reviewer")
        {
            return Err("native history terminal history does not bind receipt facts".into());
        }
    }
    Ok(())
}

fn source_role_request<'a>(
    source: &'a Map<String, Value>,
    role: &str,
    pages: &'a Value,
) -> Result<Value, String> {
    let role_source = source
        .get(role)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("native history recovery receipt must carry {role} source"))?;
    let mut request = serde_json::Map::new();
    request.insert("pages".into(), pages.clone());
    if let Some(capture) = role_source.get("capture") {
        request.insert("capture".into(), capture.clone());
    }
    Ok(Value::Object(request))
}
