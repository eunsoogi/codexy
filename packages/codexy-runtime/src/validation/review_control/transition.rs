use std::path::Path;

use serde_json::{Map, Value};

use super::{final_disposition, migration, policy, pre_pr, snapshot, state};

mod evidence;
mod genesis;

pub(super) struct PreVerdictContext<'a> {
    pub(super) repository_root: &'a Path,
    pub(super) previous_base: &'a str,
    pub(super) current_base: &'a str,
    pub(super) current: &'a Map<String, Value>,
    pub(super) prior_delta: &'a Map<String, Value>,
    pub(super) from: &'a str,
    pub(super) to: &'a str,
    pub(super) source: &'a Value,
}

pub(super) fn check_pre_verdict(context: &PreVerdictContext<'_>) -> Result<(), String> {
    evidence::pre_verdict::check(context)
}

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
    pre_pr::check_ancestry(repository_root, previous, current)?;
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
    let previous_is_native_history = previous_control.contains_key("native_history_recovery");
    if previous_count == 0 {
        genesis::check(plugin_root, previous_control)?;
        genesis::check_snapshot(previous, previous_control)?;
    } else if previous_is_current && previous_is_native_history {
        state::check_native_history_predecessor(plugin_root, previous)?;
        if current.get("nativeHistoryRecovery") != previous.get("nativeHistoryRecovery") {
            return Err("review control transition must preserve native history recovery".into());
        }
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
    let current_bound_head = current_control
        .get("final_disposition")
        .and_then(Value::as_object)
        .and_then(|disposition| disposition.get("head_oid"))
        .or_else(|| current_control.get("reviewed_head"));
    if current_bound_head != current.get("headRefOid") {
        return Err("review control transition current state must bind the current head".into());
    }
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
    pre_pr::reconcile(&normalized_control, previous_control.get("pre_pr_import"))?;

    let current_state = with_control(current, &normalized_control)?;
    state::check_pr_state(plugin_root, &current_state, false)?;

    let previous_history = history(previous_control, "previous")?;
    let current_history = history(&normalized_control, "current")?;
    let current_count = count(&normalized_control, "terminal_review_count")?;
    if final_disposition::check_transition(
        repository_root,
        previous,
        current,
        previous_control,
        &normalized_control,
        previous_count,
        current_count,
        previous_history,
        current_history,
    )? {
        return Ok(Value::Object(normalized_control));
    }
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
