use std::path::Path;

use serde_json::{Map, Value};

use super::{migration, policy, snapshot, state};

mod evidence;

pub(super) fn check_with_repository(
    plugin_root: &Path,
    repository_root: &Path,
    previous: &Value,
    current: &Value,
    current_control: &Value,
) -> Result<Value, String> {
    snapshot::check(previous, "previous")?;
    snapshot::check(current, "current")?;
    snapshot::same_pr(previous, current)?;
    snapshot::same_issue(previous, current)?;
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
    same_control_identity(previous_control, current_control)?;

    let current_profile = required_text(current_control, "profile", "current")?;
    let current_reviewer = policy::current_reviewer(plugin_root, current_profile)?;
    if current_control.get("reviewer") != Some(&current_reviewer) {
        return Err(
            "review control transition current state does not bind the selected reviewer".into(),
        );
    }

    let previous_count = count(previous_control, "terminal_review_count")?;
    let previous_reviewer = previous_control.get("reviewer");
    let legacy_reviewer = policy::legacy_reviewer(current_profile);
    let previous_is_current = previous_reviewer == Some(&current_reviewer);
    let previous_is_legacy = legacy_reviewer.as_ref() == previous_reviewer;
    if previous_count == 0 {
        check_genesis(plugin_root, previous_control)?;
        check_genesis_snapshot(previous, previous_control)?;
    } else if previous_is_current {
        state::check_pr_state(plugin_root, previous, false)?;
    } else if previous_is_legacy {
        state::check_pr_state_predecessor(plugin_root, previous)?;
    } else {
        return Err(
            "review control transition previous state does not bind an approved reviewer".into(),
        );
    }

    let mut normalized_control = current_control.clone();
    let migration = if previous_is_legacy {
        Some(migration::marker(
            current_profile,
            &current_reviewer,
            previous_count,
        )?)
    } else {
        previous_control.get("reviewer_migration").cloned()
    };
    migration::reconcile(&mut normalized_control, migration)?;

    let current_state = with_control(current, &normalized_control)?;
    state::check_pr_state(plugin_root, &current_state, false)?;

    let previous_history = history(previous_control, "previous")?;
    let current_history = history(&normalized_control, "current")?;
    let current_count = count(&normalized_control, "terminal_review_count")?;
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
            &normalized_control,
            current_history,
        )?;
    }
    Ok(Value::Object(normalized_control))
}

fn check_genesis(plugin_root: &Path, control: &Map<String, Value>) -> Result<(), String> {
    if control.get("schema").and_then(Value::as_str) != Some(state::CONTROL_SCHEMA)
        || control.contains_key("reviewed_head")
        || control.contains_key("terminal_result")
        || control.contains_key("post_cap_re_review")
        || control.contains_key("reviewer_migration")
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
    let issue = snapshot::owning_issue_number(snapshot, "previous")?;
    if issue != count(control, "issue_number")? {
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
    for field in ["issue_number", "profile"] {
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
