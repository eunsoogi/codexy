use std::path::Path;

use serde_json::{Map, Value};

use super::super::{policy, snapshot};
use super::CONTROL_SCHEMA;

const TERMINAL_RESULTS: [&str; 3] = ["PASS", "BLOCK", "UNOBSERVABLE"];
const PENDING_RESULTS: [&str; 2] = ["PENDING", "RUNNING"];
const BOOKKEEPING_FIELDS: [&str; 11] = [
    "full_review_count",
    "delta_review_count",
    "terminal_review_count",
    "terminal_review_limit",
    "terminal_review_history",
    "post_cap_re_review",
    "final_disposition",
    "reviewer_migration",
    "pre_pr_import",
    "native_history_recovery",
    "native_history_provenance",
];

pub(super) fn is_simple(control: &Map<String, Value>) -> bool {
    !BOOKKEEPING_FIELDS
        .iter()
        .any(|field| control.contains_key(*field))
}

pub(super) fn check_control(plugin_root: &Path, control: &Value) -> Result<(), String> {
    let object = control
        .as_object()
        .ok_or_else(|| "review control state must be an object".to_owned())?;
    check(plugin_root, object, None, false)
}

pub(super) fn check_pr_state(
    plugin_root: &Path,
    state: &Value,
    require_pass: bool,
) -> Result<(), String> {
    snapshot::check(state, "current")?;
    let object = state
        .get("reviewControl")
        .and_then(Value::as_object)
        .ok_or_else(|| "review control state must be an object".to_owned())?;
    check(plugin_root, object, Some(state), require_pass)
}

pub(super) fn is_terminal(plugin_root: &Path, value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    is_simple(object)
        && check(plugin_root, object, None, false).is_ok()
        && result(object).is_some_and(|result| TERMINAL_RESULTS.contains(&result))
}

pub(super) fn is_pending(plugin_root: &Path, value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    is_simple(object)
        && check(plugin_root, object, None, false).is_ok()
        && result(object).is_some_and(|result| PENDING_RESULTS.contains(&result))
}

fn check(
    plugin_root: &Path,
    control: &Map<String, Value>,
    state: Option<&Value>,
    require_pass: bool,
) -> Result<(), String> {
    if control.get("schema").and_then(Value::as_str) != Some(CONTROL_SCHEMA) {
        return Err("review control state has an unsupported schema".into());
    }
    let profile_name = control
        .get("profile")
        .and_then(Value::as_str)
        .filter(|profile| !profile.is_empty())
        .ok_or_else(|| "review control state must select a profile".to_owned())?;
    if let Some(bound) = state
        .and_then(|state| state.get("reviewProfile"))
        .and_then(Value::as_str)
        .filter(|profile| !profile.is_empty())
        && bound != profile_name
    {
        return Err("review control state profile disagrees with the selected profile".into());
    }
    let profiles =
        policy::load(plugin_root).map_err(|_| "review profile policy is unavailable".to_owned())?;
    let profile = profiles
        .get(profile_name)
        .ok_or_else(|| "review control state selects an unknown profile".to_owned())?;

    if control.contains_key("terminal_result") && control.contains_key("status") {
        return Err("review control state must use one terminal_result or status".into());
    }

    let reviewer = control.get("reviewer");
    if let Some(expected) = profile.reviewer.as_ref() {
        let expected = serde_json::to_value(expected)
            .map_err(|_| "selected reviewer is not serializable".to_owned())?;
        if reviewer != Some(&expected) {
            return Err("review control state does not bind the selected reviewer".into());
        }
    } else if reviewer.is_some_and(|reviewer| !reviewer.is_null()) {
        return Err("light review selection must not attach a reviewer".into());
    }

    let reviewed_head = control
        .get("reviewed_head")
        .or_else(|| control.get("head_oid"))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty());
    let result = result(control);
    let findings = findings(control, profile.reviewer.is_some())?;

    if profile.reviewer.is_some() {
        if result.is_none() {
            return Err("review control state must contain terminal_result or status".into());
        }
        let reviewed_head = reviewed_head
            .ok_or_else(|| "review control state must bind reviewed_head".to_owned())?;
        if let Some(state) = state {
            let current_head = state
                .get("headRefOid")
                .and_then(Value::as_str)
                .filter(|head| !head.is_empty())
                .ok_or_else(|| "review control state must bind the current head".to_owned())?;
            if reviewed_head != current_head {
                return Err(
                    "review control state reviewed_head is stale for the current head".into(),
                );
            }
            if let Some(issue_number) = control.get("issue_number") {
                let issue_number = issue_number
                    .as_u64()
                    .filter(|number| *number > 0)
                    .ok_or_else(|| {
                        "review control state issue_number must be positive".to_owned()
                    })?;
                if snapshot::owning_issue_number(state, "current")? != issue_number {
                    return Err(
                        "review control state issue_number disagrees with the owning issue".into(),
                    );
                }
            }
        }
    } else if reviewed_head.is_some() || result.is_some() || !findings.is_empty() {
        return Err("light review selection must not carry review result state".into());
    }

    if let Some(result) = result {
        if !TERMINAL_RESULTS.contains(&result) && !PENDING_RESULTS.contains(&result) {
            return Err("review control state result is invalid".into());
        }
    }
    if require_pass {
        if profile.reviewer.is_some() && result != Some("PASS") {
            return Err("review control state result is not PASS".into());
        }
        if !findings.is_empty() {
            return Err("review control state has unresolved actionable findings".into());
        }
    }
    Ok(())
}

fn result(control: &Map<String, Value>) -> Option<&str> {
    control
        .get("terminal_result")
        .or_else(|| control.get("status"))
        .and_then(Value::as_str)
}

fn findings(control: &Map<String, Value>, required: bool) -> Result<Vec<Value>, String> {
    let Some(value) = control.get("unresolved_findings") else {
        if required {
            return Err("review control state must contain unresolved_findings".into());
        }
        return Ok(Vec::new());
    };
    value
        .as_array()
        .cloned()
        .ok_or_else(|| "review control state unresolved_findings must be an array".to_owned())
}
