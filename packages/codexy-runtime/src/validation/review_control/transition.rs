use std::path::Path;

use serde_json::{Map, Value};

use super::{policy, snapshot, state};

mod evidence;

pub(super) fn check_with_repository(
    plugin_root: &Path,
    repository_root: &Path,
    previous: &Value,
    current: &Value,
    current_control: &Value,
) -> Result<(), String> {
    snapshot::check(previous, "previous")?;
    snapshot::check(current, "current")?;
    snapshot::same_pr(previous, current)?;
    if current.get("reviewControl").is_some() {
        return Err(
            "review control current PR snapshot must not carry a caller-supplied predecessor"
                .into(),
        );
    }
    let previous_control = snapshot_control(previous, "previous")?;
    let current_control = current_control
        .as_object()
        .ok_or_else(|| "review control current state must be an object".to_owned())?;
    let current_state = with_control(current, current_control)?;
    state::check(plugin_root, &current_state, false)?;

    let previous_count = count(previous_control, "terminal_review_count")?;
    if previous_count == 0 {
        check_genesis(plugin_root, previous_control)?;
        check_genesis_snapshot(previous, previous_control)?;
    } else {
        state::check(plugin_root, previous, false)?;
    }
    same_control_identity(previous_control, current_control)?;

    let previous_history = history(previous_control, "previous")?;
    let current_history = history(current_control, "current")?;
    let current_count = count(current_control, "terminal_review_count")?;
    let Some(expected_count) = previous_count.checked_add(1) else {
        return Err("review control transition terminal count overflow".into());
    };
    if current_count != expected_count {
        return Err("review control transition must append exactly one terminal event".into());
    }
    if current_history.len() != previous_history.len() + 1
        || current_history.get(..previous_history.len()) != Some(previous_history)
    {
        return Err("review control transition must preserve the prior terminal history".into());
    }
    if current_count == 3 {
        evidence::check(
            repository_root,
            previous,
            current,
            current_control,
            current_history,
        )?;
    }
    Ok(())
}

fn check_genesis(plugin_root: &Path, control: &Map<String, Value>) -> Result<(), String> {
    if control.get("schema").and_then(Value::as_str) != Some(state::CONTROL_SCHEMA)
        || control.contains_key("reviewed_head")
        || control.contains_key("terminal_result")
        || control.contains_key("post_cap_re_review")
        || count(control, "full_review_count")? != 0
        || count(control, "delta_review_count")? != 0
        || count(control, "terminal_review_count")? != 0
        || !history(control, "genesis")?.is_empty()
    {
        return Err("review control transition previous state is not a clean genesis".into());
    }
    if !control
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        return Err("review control transition genesis must have no findings".into());
    }
    let profile_name = required_text(control, "profile", "genesis")?;
    let profiles =
        policy::load(plugin_root).map_err(|_| "review profile policy is unavailable".to_owned())?;
    let profile = profiles
        .get(profile_name)
        .ok_or_else(|| "review control transition genesis selects an unknown profile".to_owned())?;
    let reviewer = profile
        .reviewer
        .as_ref()
        .ok_or_else(|| "review control transition genesis must select a reviewer".to_owned())?;
    let expected = serde_json::to_value(reviewer)
        .map_err(|_| "review control transition reviewer is not serializable".to_owned())?;
    if control.get("reviewer") != Some(&expected)
        || count(control, "terminal_review_limit")? != u64::from(profile.terminal_review_limit)
    {
        return Err("review control transition genesis does not bind policy".into());
    }
    Ok(())
}

fn check_genesis_snapshot(snapshot: &Value, control: &Map<String, Value>) -> Result<(), String> {
    let object = snapshot
        .as_object()
        .ok_or_else(|| "previous PR snapshot must be an object".to_owned())?;
    let issue = count(control, "issue_number")?;
    if object.get("number").and_then(Value::as_u64) != Some(issue) {
        return Err("genesis PR snapshot issue identity disagrees with review control".into());
    }
    if let Some(profile) = object.get("reviewProfile").and_then(Value::as_str) {
        if control.get("profile").and_then(Value::as_str) != Some(profile) {
            return Err("genesis PR snapshot profile disagrees with review control".into());
        }
    }
    Ok(())
}

fn same_control_identity(
    previous: &Map<String, Value>,
    current: &Map<String, Value>,
) -> Result<(), String> {
    for field in ["issue_number", "profile", "reviewer"] {
        if previous.get(field) != current.get(field) {
            return Err(format!("review control transition changes {field}"));
        }
    }
    Ok(())
}

fn with_control(snapshot: &Value, control: &Map<String, Value>) -> Result<Value, String> {
    let mut value = snapshot.clone();
    let object = value
        .as_object_mut()
        .ok_or_else(|| "current PR snapshot must be an object".to_owned())?;
    object.insert("reviewControl".into(), Value::Object(control.clone()));
    Ok(value)
}

fn snapshot_control<'a>(
    snapshot: &'a Value,
    label: &str,
) -> Result<&'a Map<String, Value>, String> {
    snapshot
        .get("reviewControl")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("review control {label} PR snapshot must carry reviewControl"))
}

fn history<'a>(control: &'a Map<String, Value>, label: &str) -> Result<&'a [Value], String> {
    control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("review control {label} state must carry terminal history"))
}

fn required_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("review control {label} must contain non-empty {key}"))
}

fn count(control: &Map<String, Value>, key: &str) -> Result<u64, String> {
    control
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("review control state must contain numeric {key}"))
}
