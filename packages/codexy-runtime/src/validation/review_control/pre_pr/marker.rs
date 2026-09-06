use std::{collections::HashMap, path::Path};

use serde_json::{Map, Value};

use super::{
    IMPORT_SCHEMA, check_ancestor, check_issue, check_source, number, object, reject_unknown, text,
};

pub(crate) fn check_state(
    pr_snapshot: bool,
    require_pass: bool,
    head: &str,
    reviewed_head: &str,
    control: &Map<String, Value>,
) -> Result<(), String> {
    if control.contains_key("pre_pr_import") {
        check_marker(control)?;
    }
    if pr_snapshot && reviewed_head != head {
        if !control.contains_key("pre_pr_import") {
            return Err("review control state reviewed_head is stale".into());
        }
        if require_pass {
            return Err("pre-PR review history is not current-head readiness".into());
        }
    }
    Ok(())
}

pub(crate) fn reconcile(
    current: &Map<String, Value>,
    previous: Option<&Value>,
) -> Result<(), String> {
    match (current.get("pre_pr_import"), previous) {
        (Some(actual), Some(expected)) if actual == expected => Ok(()),
        (Some(_), Some(_)) => {
            Err("review control transition changes pre-PR import provenance".into())
        }
        (Some(_), None) => Err("review control transition adds pre-PR import provenance".into()),
        (None, Some(_)) => Err("review control transition removes pre-PR import provenance".into()),
        (None, None) => Ok(()),
    }
}

pub(crate) fn check_ancestry(
    repository_root: &Path,
    previous: &Value,
    current: &Value,
) -> Result<(), String> {
    let Some(control) = previous.get("reviewControl").and_then(Value::as_object) else {
        return Ok(());
    };
    if !control.contains_key("pre_pr_import") {
        return Ok(());
    }
    let reviewed = text(control, "reviewed_head", "pre-PR import")?;
    let current_head = text(
        current
            .as_object()
            .ok_or_else(|| "current PR snapshot must be an object".to_owned())?,
        "headRefOid",
        "current PR snapshot",
    )?;
    check_ancestor(repository_root, reviewed, current_head, "pre-PR import")
}

pub(crate) fn check_marker(control: &Map<String, Value>) -> Result<(), String> {
    let marker = object(control.get("pre_pr_import"), "pre-PR import marker")?;
    reject_unknown(
        marker,
        &["schema", "source", "issue", "complete", "events"],
        "pre-PR import marker",
    )?;
    if text(marker, "schema", "pre-PR import marker")? != IMPORT_SCHEMA
        || marker.get("complete") != Some(&Value::Bool(true))
    {
        return Err("pre-PR import marker is incomplete or has an unsupported schema".into());
    }
    check_source(object(marker.get("source"), "marker source")?)?;
    let issue_number = check_issue(object(marker.get("issue"), "marker issue")?)?;
    if control.get("issue_number").and_then(Value::as_u64) != Some(issue_number) {
        return Err("pre-PR import marker changes the owning issue".into());
    }
    let refs = marker
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| "pre-PR import marker must carry event references".to_owned())?;
    let history = control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .ok_or_else(|| "pre-PR import marker requires terminal history".to_owned())?;
    if refs.is_empty() || refs.len() > 2 || refs.len() > history.len() {
        return Err("pre-PR import marker has an invalid event prefix".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut turns = std::collections::HashSet::new();
    let mut ordinals = HashMap::new();
    for (index, value) in refs.iter().enumerate() {
        let reference = object(Some(value), "pre-PR event reference")?;
        reject_unknown(
            reference,
            &["id", "thread_id", "turn_id", "ordinal"],
            "pre-PR event reference",
        )?;
        if reference.get("id") != history[index].get("id") {
            return Err("pre-PR import marker does not bind the history prefix".into());
        }
        let id = text(reference, "id", "pre-PR event reference")?;
        let thread_id = text(reference, "thread_id", "pre-PR event reference")?;
        let turn = text(reference, "turn_id", "pre-PR event reference")?;
        if !ids.insert(id) || !turns.insert(turn) {
            return Err("pre-PR import marker has duplicate event references".into());
        }
        let next = number(reference, "ordinal", "pre-PR event reference")?;
        if ordinals.get(thread_id).is_some_and(|last| next <= *last) {
            return Err("pre-PR import marker event order is not increasing".into());
        }
        ordinals.insert(thread_id.to_owned(), next);
    }
    Ok(())
}
