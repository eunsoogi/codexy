use std::path::Path;

use serde_json::{Value, json};

use super::{CONTROL_SCHEMA, check};

pub(super) fn is_terminal(plugin_root: &Path, record: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(record) else {
        return false;
    };
    let Some(head) = value
        .get("reviewed_head")
        .or_else(|| value.get("head_oid"))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
    else {
        return false;
    };
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
        "reviewed_head": head,
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
    check(
        plugin_root,
        &json!({"headRefOid": head, "reviewControl": control}),
        false,
    )
    .is_ok()
}
