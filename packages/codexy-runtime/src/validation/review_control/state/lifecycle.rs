use std::path::Path;

use serde_json::{Value, json};

use super::{CONTROL_SCHEMA, ReviewerMode, StateSource, check};

pub(super) fn is_terminal(plugin_root: &Path, record: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(record) else {
        return false;
    };
    let Some(reviewed_head) = value
        .get("reviewed_head")
        .or_else(|| value.get("head_oid"))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
    else {
        return false;
    };
    let admission_head = value
        .get("final_disposition")
        .and_then(Value::as_object)
        .and_then(|disposition| disposition.get("head_oid"))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
        .unwrap_or(reviewed_head);
    let Some(issue_number) = value.get("issue_number").and_then(Value::as_u64) else {
        return false;
    };
    let Some(profile) = value.get("profile").and_then(Value::as_str) else {
        return false;
    };
    let Some(unresolved_findings) = value.get("unresolved_findings").cloned() else {
        return false;
    };
    let Some(full_review_count) = value.get("full_review_count").and_then(Value::as_u64) else {
        return false;
    };
    let Some(delta_review_count) = value.get("delta_review_count").and_then(Value::as_u64) else {
        return false;
    };
    let Some(terminal_review_count) = value.get("terminal_review_count").and_then(Value::as_u64)
    else {
        return false;
    };
    let Some(terminal_review_limit) = value.get("terminal_review_limit").and_then(Value::as_u64)
    else {
        return false;
    };
    let Some(history) = value.get("terminal_review_history").cloned() else {
        return false;
    };
    let terminal = match value
        .get("terminal_result")
        .or_else(|| value.get("state"))
        .and_then(Value::as_str)
    {
        Some("PASS") => "PASS",
        Some("BLOCK") => "BLOCK",
        Some("UNOBSERVABLE") => "UNOBSERVABLE",
        _ => return false,
    };
    let mut control = json!({
        "schema": CONTROL_SCHEMA,
        "issue_number": issue_number,
        "profile": profile,
        "reviewer": value.get("reviewer").cloned().unwrap_or(Value::Null),
        "reviewed_head": reviewed_head,
        "terminal_result": terminal,
        "unresolved_findings": unresolved_findings,
        "full_review_count": full_review_count,
        "delta_review_count": delta_review_count,
        "terminal_review_count": terminal_review_count,
        "terminal_review_limit": terminal_review_limit,
        "terminal_review_history": history,
    });
    if let Some(post_cap) = value.get("post_cap_re_review") {
        control["post_cap_re_review"] = post_cap.clone();
    }
    if let Some(migration) = value.get("reviewer_migration") {
        control["reviewer_migration"] = migration.clone();
    }
    if let Some(provenance) = value.get("native_history_provenance") {
        control["native_history_provenance"] = provenance.clone();
    }
    if let Some(disposition) = value.get("final_disposition") {
        control["final_disposition"] = disposition.clone();
    }
    check::with_mode(
        plugin_root,
        &json!({"headRefOid": admission_head, "reviewControl": control}),
        false,
        ReviewerMode::Current,
        StateSource::ControlOnly,
    )
    .is_ok()
}
