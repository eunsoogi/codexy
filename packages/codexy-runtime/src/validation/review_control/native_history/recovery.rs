use std::path::Path;

use serde_json::Value;

use super::{RECEIPT_SCHEMA, bind_current_pr_snapshot, normalize_native_history};

pub(super) fn recover_text(
    plugin_root: &Path,
    current_text: &str,
    input_text: &str,
) -> Result<Value, String> {
    let current = serde_json::from_str(current_text)
        .map_err(|error| format!("native history current PR state is invalid: {error}"))?;
    let input = serde_json::from_str(input_text)
        .map_err(|error| format!("native history input is invalid: {error}"))?;
    recover(plugin_root, &current, &input)
}

fn recover(plugin_root: &Path, current: &Value, input: &Value) -> Result<Value, String> {
    super::super::snapshot::check(current, "current")?;
    let current_map = current
        .as_object()
        .ok_or("current PR snapshot must be an object")?;
    if current_map.contains_key("reviewControl") {
        return Err("native history recovery must not replace existing reviewControl".into());
    }
    if current_map.contains_key("nativeHistoryRecovery") {
        return Err("native history recovery must not replace existing recovery receipt".into());
    }
    if input.get("currentPrSnapshot").is_some() {
        return Err("native history input must not supply the current PR snapshot".into());
    }
    let profile = current_map
        .get("reviewProfile")
        .and_then(Value::as_str)
        .filter(|profile| !profile.is_empty())
        .ok_or("current PR snapshot must carry reviewProfile")?;
    let reviewer = super::super::policy::current_reviewer(plugin_root, profile)?;
    let receipt = normalize_native_history(input)?;
    let bound = bind_current_pr_snapshot(&receipt, current)?;
    let admission = bound
        .get("admission")
        .and_then(Value::as_object)
        .ok_or("native history receipt must contain admission")?;
    if admission.get("temporal").and_then(Value::as_str) != Some("proved_post_pr") {
        return Err("native history recovery requires proved_post_pr timing".into());
    }
    let events = bound
        .get("events")
        .and_then(Value::as_array)
        .ok_or("native history receipt must contain events")?;
    if events.is_empty() || events.len() > 2 {
        return Err("native history recovery accepts one full event and one optional delta".into());
    }
    for (index, event) in events.iter().enumerate() {
        let expected = if index == 0 { "full" } else { "delta" };
        if event.get("kind").and_then(Value::as_str) != Some(expected) {
            return Err("native history recovery must be ordered full then optional delta".into());
        }
    }
    let issue_number = super::super::snapshot::owning_issue_number(current, "current")?;
    let target_issue = bound
        .get("target")
        .and_then(Value::as_object)
        .and_then(|target| target.get("owningIssue"))
        .and_then(Value::as_u64);
    if target_issue != Some(issue_number) {
        return Err("native history target owning issue does not match current PR".into());
    }
    let history = events
        .iter()
        .map(|event| {
            Ok(serde_json::json!({
                "id": event.get("id").cloned().ok_or("recovered event id is missing")?,
                "kind": event.get("kind").cloned().ok_or("recovered event kind is missing")?,
                "reviewer": event.get("reviewer").cloned().ok_or("recovered source reviewer is missing")?,
                "policy_reviewer": reviewer.clone(),
                "source_reviewer": event.get("reviewer").cloned().ok_or("recovered source reviewer is missing")?,
                "reviewed_head": event.get("reviewed_head").cloned().ok_or("recovered reviewed head is missing")?,
                "terminal_result": event.get("terminal_result").cloned().ok_or("recovered terminal result is missing")?,
                "unresolved_findings": event.get("findings").cloned().ok_or("recovered findings are missing")?
            }))
        })
        .collect::<Result<Vec<Value>, String>>()?;
    let last = events
        .last()
        .ok_or("native history recovery has no events")?;
    let full_count = events
        .iter()
        .filter(|event| event["kind"] == "full")
        .count();
    let delta_count = events
        .iter()
        .filter(|event| event["kind"] == "delta")
        .count();
    let event_ids = events
        .iter()
        .map(|event| {
            event
                .get("id")
                .cloned()
                .ok_or("recovered event id is missing")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let control = serde_json::json!({
        "schema": super::super::state::CONTROL_SCHEMA,
        "issue_number": issue_number,
        "profile": profile,
        "reviewer": reviewer,
        "reviewed_head": last["reviewed_head"],
        "terminal_result": last["terminal_result"],
        "unresolved_findings": last["findings"],
        "full_review_count": full_count,
        "delta_review_count": delta_count,
        "terminal_review_count": events.len(),
        "terminal_review_limit": super::super::policy::terminal_review_limit(plugin_root, profile)?,
        "terminal_review_history": history,
        "native_history_recovery": {
            "schema": RECEIPT_SCHEMA,
            "temporal": "proved_post_pr",
            "event_ids": event_ids.clone()
        },
        "native_history_provenance": {
            "schema": RECEIPT_SCHEMA,
            "temporal": "proved_post_pr",
            "event_ids": event_ids
        }
    });
    super::super::state::check_control(plugin_root, &control)?;
    let mut state = current.clone();
    let state_map = state
        .as_object_mut()
        .ok_or("current PR snapshot must be an object")?;
    state_map.insert("reviewControl".into(), control);
    state_map.insert("nativeHistoryRecovery".into(), bound);
    Ok(state)
}
