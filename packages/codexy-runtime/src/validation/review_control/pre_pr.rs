use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use serde_json::{Value, json};

use super::{migration, policy, snapshot, state};

#[path = "pre_pr/input.rs"]
mod input;
#[path = "pre_pr/marker.rs"]
mod marker;

pub(super) use input::{
    check_ancestor, check_issue, check_source, number, object, reject_unknown, text,
};
pub(super) use marker::{check_ancestry, check_state, reconcile};

pub(super) const IMPORT_SCHEMA: &str = "codexy.review-control-pre-pr-history.v1";
const RESULTS: [&str; 3] = ["PASS", "BLOCK", "UNOBSERVABLE"];

pub(super) fn import(
    plugin_root: &Path,
    repository_root: &Path,
    current: &Value,
    envelope: &Value,
) -> Result<Value, String> {
    snapshot::check(current, "current")?;
    if current.get("reviewControl").is_some() {
        return Err("pre-PR review history import is genesis-only".into());
    }
    let envelope = object(Some(envelope), "pre-PR history envelope")?;
    reject_unknown(
        envelope,
        &[
            "schema",
            "source",
            "issue",
            "profile",
            "complete",
            "terminal_event_count",
            "events",
        ],
        "pre-PR history envelope",
    )?;
    if text(envelope, "schema", "envelope")? != IMPORT_SCHEMA
        || envelope.get("complete") != Some(&Value::Bool(true))
    {
        return Err("pre-PR history source is incomplete or has an unsupported schema".into());
    }
    let source = object(envelope.get("source"), "source")?;
    check_source(source)?;
    let issue = object(envelope.get("issue"), "owning issue")?;
    let issue_number = check_issue(issue)?;
    let current_issue = current
        .get("capture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("owningIssue"))
        .and_then(Value::as_object)
        .ok_or_else(|| "current PR snapshot must carry owning issue identity".to_owned())?;
    for key in ["repository", "number", "url"] {
        if issue.get(key) != current_issue.get(key) {
            return Err("pre-PR history issue does not bind the current PR owning issue".into());
        }
    }
    let profile_name = text(envelope, "profile", "envelope")?;
    let reviewer = policy::current_reviewer(plugin_root, profile_name)?;
    let legacy_reviewer = policy::legacy_reviewer(profile_name);
    let events = envelope
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| "pre-PR history envelope must carry events".to_owned())?;
    let count = envelope
        .get("terminal_event_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "pre-PR history envelope must contain terminal_event_count".to_owned())?;
    if count == 0 || usize::try_from(count).ok() != Some(events.len()) || events.len() > 2 {
        return Err("pre-PR history envelope has an invalid terminal event count".into());
    }
    if let Some(bound) = current.get("reviewProfile").and_then(Value::as_str) {
        if bound != profile_name {
            return Err("pre-PR history profile disagrees with the current PR".into());
        }
    }
    let current_head = snapshot::required_oid(
        current
            .as_object()
            .ok_or_else(|| "current PR snapshot must be an object".to_owned())?,
        "headRefOid",
        "current",
    )?;
    let mut history = Vec::with_capacity(events.len());
    let mut refs = Vec::with_capacity(events.len());
    let mut ids = HashSet::new();
    let mut turns = HashSet::new();
    let mut last_ordinals = HashMap::new();
    let mut previous_head = None;
    let mut has_legacy_prefix = false;
    for (index, value) in events.iter().enumerate() {
        let event = object(Some(value), "review event")?;
        reject_unknown(
            event,
            &[
                "sequence",
                "id",
                "thread_id",
                "turn_id",
                "ordinal",
                "turn_status",
                "item_type",
                "phase",
                "reviewer",
                "kind",
                "reviewed_head",
                "terminal_result",
                "unresolved_findings",
            ],
            "review event",
        )?;
        if number(event, "sequence", "review event")? != index as u64
            || text(event, "turn_status", "review event")? != "completed"
            || text(event, "item_type", "review event")? != "AgentMessage"
            || text(event, "phase", "review event")? != "final_answer"
        {
            return Err("pre-PR review event is not a completed final reviewer message".into());
        }
        let thread_id = text(event, "thread_id", "review event")?;
        let id = text(event, "id", "review event")?;
        let turn = text(event, "turn_id", "review event")?;
        if !ids.insert(id) || !turns.insert(turn) {
            return Err("pre-PR review history has duplicate event identity".into());
        }
        let ordinal = number(event, "ordinal", "review event")?;
        if last_ordinals
            .get(thread_id)
            .is_some_and(|last| ordinal <= *last)
        {
            return Err("pre-PR review history event order is not increasing".into());
        }
        last_ordinals.insert(thread_id.to_owned(), ordinal);
        let kind = text(event, "kind", "review event")?;
        if kind != if index == 0 { "full" } else { "delta" } {
            return Err("pre-PR review history must be ordered full then optional delta".into());
        }
        let event_reviewer = event
            .get("reviewer")
            .ok_or_else(|| "review event must contain reviewer facts".to_owned())?;
        let is_legacy = index == 0 && legacy_reviewer.as_ref() == Some(event_reviewer);
        if event_reviewer != &reviewer && !is_legacy {
            return Err("pre-PR review event does not bind the selected reviewer policy".into());
        }
        has_legacy_prefix |= is_legacy;
        let head = snapshot::required_oid(event, "reviewed_head", "review event")?;
        if let Some(previous) = previous_head {
            check_ancestor(repository_root, previous, head, "ordered review history")?;
        }
        check_ancestor(repository_root, head, current_head, "current PR head")?;
        previous_head = Some(head);
        let result = text(event, "terminal_result", "review event")?;
        if !RESULTS.contains(&result)
            || event
                .get("unresolved_findings")
                .and_then(Value::as_array)
                .is_none()
        {
            return Err("pre-PR review event has invalid terminal facts".into());
        }
        history.push(json!({
            "id": id,
            "kind": kind,
            "reviewer": event_reviewer,
            "reviewed_head": head,
            "terminal_result": result,
            "unresolved_findings": event["unresolved_findings"]
        }));
        refs.push(json!({"id": id, "thread_id": thread_id, "turn_id": turn, "ordinal": ordinal}));
    }
    let last = history
        .last()
        .ok_or_else(|| "pre-PR review history is empty".to_owned())?;
    let full_count = history
        .iter()
        .filter(|event| event["kind"] == "full")
        .count();
    let delta_count = history
        .iter()
        .filter(|event| event["kind"] == "delta")
        .count();
    let mut control = json!({
        "schema": state::CONTROL_SCHEMA,
        "issue_number": issue_number,
        "profile": profile_name,
        "reviewer": reviewer,
        "reviewed_head": last["reviewed_head"],
        "terminal_result": last["terminal_result"],
        "unresolved_findings": last["unresolved_findings"],
        "full_review_count": full_count,
        "delta_review_count": delta_count,
        "terminal_review_count": count,
        "terminal_review_limit": policy::terminal_review_limit(plugin_root, profile_name)?,
        "terminal_review_history": history,
        "pre_pr_import": {"schema": IMPORT_SCHEMA, "source": source, "issue": issue, "complete": true, "events": refs}
    });
    if has_legacy_prefix {
        control["reviewer_migration"] = migration::marker(profile_name, &reviewer, 1)?;
    }
    state::check_control(plugin_root, &control)?;
    let mut state = current.clone();
    state["reviewControl"] = control;
    Ok(state)
}
