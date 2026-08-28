use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

mod classification;
mod policy;

const CONTROL_SCHEMA: &str = "codexy.review-control-state.v1";
const TERMINAL_RESULTS: [&str; 3] = ["PASS", "BLOCK", "UNOBSERVABLE"];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    policy::load(plugin_root)
        .and_then(|_| classification::check(plugin_root))
        .map_or_else(|error| vec![error.to_string()], |_| Vec::new())
}

pub(super) fn resolve_profile(plugin_root: &Path, request: &str) -> Result<Value> {
    policy::resolve(plugin_root, request)
}

pub(super) fn check_packet(
    _plugin_root: &Path,
    _repository_root: &Path,
    _legacy_output: &Path,
    _legacy_input: &str,
) -> Result<()> {
    Ok(())
}

pub(super) fn check_economics(
    _plugin_root: &Path,
    _repository_root: &Path,
    _legacy_input: &str,
) -> Result<()> {
    Ok(())
}

pub(super) fn check_handoff(plugin_root: &Path, state: &Value) -> Vec<String> {
    check_state(plugin_root, state, true)
        .err()
        .into_iter()
        .collect()
}

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
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
    let Some(profile) = value.get("profile").and_then(Value::as_str) else {
        return false;
    };
    let terminal = match value
        .get("terminal_result")
        .or_else(|| value.get("state"))
        .and_then(Value::as_str)
    {
        Some("PASS" | "passed") => "PASS",
        Some("BLOCK" | "blocked") => "BLOCK",
        Some("UNOBSERVABLE" | "unobservable") => "UNOBSERVABLE",
        _ => return false,
    };
    let control = serde_json::json!({
        "schema": CONTROL_SCHEMA,
        "profile": profile,
        "reviewer": value.get("reviewer").cloned().unwrap_or(Value::Null),
        "reviewed_head": head,
        "terminal_result": terminal,
        "unresolved_findings": value.get("unresolved_findings").cloned().unwrap_or_else(|| value.get("blockers").cloned().unwrap_or_else(|| serde_json::json!([]))),
        "full_review_count": value.get("full_review_count").and_then(Value::as_u64).unwrap_or(1),
        "delta_review_count": value.get("delta_review_count").and_then(Value::as_u64).unwrap_or(0),
    });
    check_state(
        plugin_root,
        &serde_json::json!({"headRefOid": head, "reviewControl": control}),
        true,
    )
    .is_ok()
}

pub(super) fn build_pr_state(
    plugin_root: &Path,
    base_text: &str,
    control_text: &str,
) -> Result<Value> {
    let mut state: Value = serde_json::from_str(base_text)?;
    let control: Value = serde_json::from_str(control_text)?;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("base PR state must be an object"))?;
    if object.contains_key("reviewControl") {
        bail!("base PR state must not already contain review control fields");
    }
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    object.insert("reviewControl".into(), control);
    check_state(plugin_root, &state, false).map_err(anyhow::Error::msg)?;
    Ok(state)
}

pub(super) fn produce(
    _plugin_root: &Path,
    _repository_root: &Path,
    request_text: &str,
) -> Result<Value> {
    let request: Value = serde_json::from_str(request_text)
        .map_err(|error| anyhow::anyhow!("review control input is invalid: {error}"))?;
    let control = request
        .get("control_state")
        .or_else(|| request.get("reviewControl"))
        .cloned()
        .unwrap_or(request);
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    Ok(serde_json::json!({"control_state": control}))
}

fn check_state(plugin_root: &Path, state: &Value, require_pass: bool) -> Result<(), String> {
    let head = non_empty_string(state, "headRefOid")
        .ok_or_else(|| "review control state must bind the current head".to_owned())?;
    let control = state
        .get("reviewControl")
        .and_then(Value::as_object)
        .ok_or_else(|| "review control state must be an object".to_owned())?;
    if control.get("schema").and_then(Value::as_str) != Some(CONTROL_SCHEMA) {
        return Err("review control state has an unsupported schema".into());
    }
    let selected = control
        .get("profile")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "review control state must select a profile".to_owned())?;
    if let Some(bound_profile) = state
        .get("reviewProfile")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        if bound_profile != selected {
            return Err("review control state profile disagrees with the selected profile".into());
        }
    }
    let profiles =
        policy::load(plugin_root).map_err(|_| "review profile policy is unavailable".to_owned())?;
    let profile = profiles
        .get(selected)
        .ok_or_else(|| "review control state selects an unknown profile".to_owned())?;

    if profile.reviewer.is_none() {
        if control
            .get("reviewer")
            .is_some_and(|reviewer| !reviewer.is_null())
        {
            return Err("light review selection must not attach a reviewer".into());
        }
        return Ok(());
    }

    let expected_reviewer = serde_json::to_value(profile.reviewer.as_ref().expect("reviewer"))
        .map_err(|_| "selected reviewer is not serializable".to_owned())?;
    if control.get("reviewer") != Some(&expected_reviewer) {
        return Err("review control state does not bind the selected reviewer".into());
    }
    let reviewed_head = direct_string(control, "reviewed_head", "head_oid")
        .ok_or_else(|| "review control state must bind reviewed_head".to_owned())?;
    if reviewed_head != head {
        return Err("review control state reviewed_head is stale".into());
    }
    let terminal = control
        .get("terminal_result")
        .and_then(Value::as_str)
        .ok_or_else(|| "review control state must name terminal_result".to_owned())?;
    if !TERMINAL_RESULTS.contains(&terminal) {
        return Err("review control state terminal_result is invalid".into());
    }
    let findings = control
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "review control state must list unresolved_findings".to_owned())?;
    let full_count = count(control, "full_review_count")?;
    let delta_count = count(control, "delta_review_count")?;
    if full_count != 1 || delta_count > 1 {
        return Err("review control state exceeds the bounded review cycle".into());
    }
    if require_pass && terminal != "PASS" {
        return Err("review control state terminal_result is not PASS".into());
    }
    if require_pass && !findings.is_empty() {
        return Err("review control state has unresolved actionable findings".into());
    }
    Ok(())
}

fn non_empty_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

fn direct_string<'a>(
    value: &'a serde_json::Map<String, Value>,
    primary: &str,
    alias: &str,
) -> Option<&'a str> {
    value
        .get(primary)
        .or_else(|| value.get(alias))
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

fn count(value: &serde_json::Map<String, Value>, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("review control state must contain numeric {key}"))
}
