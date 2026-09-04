use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

mod classification;
mod history;
mod policy;
mod snapshot;
mod state;
mod transition;

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
    state::check(plugin_root, state, true)
        .err()
        .into_iter()
        .collect()
}

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
    state::is_lifecycle_terminal(plugin_root, record)
}

pub(super) fn build_pr_state(
    plugin_root: &Path,
    repository_root: &Path,
    current_text: &str,
    control_text: &str,
    previous_text: &str,
) -> Result<Value> {
    let current: Value = serde_json::from_str(current_text)?;
    let control: Value = serde_json::from_str(control_text)?;
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    let previous: Value = serde_json::from_str(previous_text)
        .map_err(|error| anyhow::anyhow!("previous PR state is invalid: {error}"))?;
    if control.get("profile").and_then(Value::as_str) != Some("light") {
        transition::check_with_repository(
            plugin_root,
            repository_root,
            &previous,
            &current,
            &control,
        )
        .map_err(anyhow::Error::msg)?;
    }
    let mut state = current;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("current PR state must be an object"))?;
    object.insert("reviewControl".into(), control);
    state::check(plugin_root, &state, false).map_err(anyhow::Error::msg)?;
    Ok(state)
}

pub(super) fn produce(
    plugin_root: &Path,
    repository_root: &Path,
    request_text: &str,
) -> Result<Value> {
    let request: Value = serde_json::from_str(request_text)
        .map_err(|error| anyhow::anyhow!("review control input is invalid: {error}"))?;
    let control = request
        .get("control_state")
        .or_else(|| request.get("reviewControl"))
        .cloned()
        .unwrap_or_else(|| request.clone());
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    if request.get("previous_control_state").is_some() {
        bail!(
            "review control producer must derive prior state from previous_pr_state, not previous_control_state"
        );
    }
    state::check_control(plugin_root, &control).map_err(anyhow::Error::msg)?;
    if control
        .get("profile")
        .and_then(Value::as_str)
        .is_some_and(|profile| profile != "light")
    {
        let current = request
            .get("current_pr_state")
            .ok_or_else(|| anyhow::anyhow!("review control producer requires current_pr_state"))?;
        let previous = request
            .get("previous_pr_state")
            .ok_or_else(|| anyhow::anyhow!("review control producer requires previous_pr_state"))?;
        transition::check_with_repository(
            plugin_root,
            repository_root,
            previous,
            current,
            &control,
        )
        .map_err(anyhow::Error::msg)?;
    }
    Ok(serde_json::json!({"control_state": control}))
}
