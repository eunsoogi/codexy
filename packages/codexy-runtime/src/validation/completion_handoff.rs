use serde_json::Value;
use std::path::Path;

pub(super) fn check(plugin_root: &Path, handoff: &str, pr_state: &str) -> Vec<String> {
    let state = match serde_json::from_str::<Value>(pr_state) {
        Ok(value) => value,
        Err(error) => return vec![format!("completion handoff PR state JSON error: {error}")],
    };
    if let Some(error) = pr_state_input_error(&state) {
        return vec![error];
    }
    if has_direct_control(&state) {
        let direct_errors = super::review_control::check_handoff(plugin_root, &state);
        if !direct_errors.is_empty() {
            return direct_errors;
        }
    }
    let loc_errors = super::completion_handoff_loc_remediation::check(handoff);
    if !loc_errors.is_empty() {
        return loc_errors;
    }
    let child_errors = super::child_handoff_readiness::check(handoff, &state);
    if !child_errors.is_empty() {
        return child_errors;
    }
    let compaction_errors = super::completion_handoff_compaction::check(handoff, &state);
    if !compaction_errors.is_empty() {
        return compaction_errors;
    }
    if let Some(error) = super::review_thread_readiness::check_handoff(handoff, &state) {
        return vec![error];
    }
    let review_thread_errors = super::review_thread_resolution::check(handoff, &state);
    if !review_thread_errors.is_empty() {
        return review_thread_errors;
    }
    if let Some(error) = super::completion_handoff_waiting::check(handoff) {
        return vec![error];
    }
    let normalized = handoff.to_ascii_lowercase();
    if let Some(error) = super::completion_handoff_pending_worktree::check(&normalized) {
        return vec![error];
    }
    Vec::new()
}

fn has_direct_control(state: &Value) -> bool {
    let Some(control) = state.get("reviewControl").and_then(Value::as_object) else {
        return false;
    };
    !["decision", "evidence", "ledger"]
        .iter()
        .any(|field| control.contains_key(*field))
}

fn pr_state_input_error(state: &Value) -> Option<String> {
    ["state", "mergeStateStatus"]
        .iter()
        .find(|field| string_field(state, field).is_none())
        .map(|field| format!("completion handoff PR state missing required field: {field}"))
        .or_else(|| {
            state
                .get("isDraft")
                .and_then(Value::as_bool)
                .is_none()
                .then_some("completion handoff PR state missing required field: isDraft".into())
        })
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}
