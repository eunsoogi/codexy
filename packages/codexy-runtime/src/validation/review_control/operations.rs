use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

use super::{request, snapshot, state};

pub(crate) fn build_pr_state(
    plugin_root: &Path,
    _repository_root: &Path,
    current_text: &str,
    control_text: &str,
    previous_text: &str,
) -> Result<Value> {
    let control: Value = serde_json::from_str(control_text)?;
    request::reject_retired_inputs(&control).map_err(anyhow::Error::msg)?;
    request::reject_retired_control(&control).map_err(anyhow::Error::msg)?;
    if !control.is_object() {
        bail!("review control state must be an object");
    }

    let raw_current: Value = serde_json::from_str(current_text)?;
    request::reject_retired_inputs(&raw_current).map_err(anyhow::Error::msg)?;
    let raw_previous = serde_json::from_str::<Value>(previous_text).ok();
    if let Some(previous) = raw_previous.as_ref() {
        request::reject_retired_inputs(previous).map_err(anyhow::Error::msg)?;
    }

    let current = snapshot::normalize(&raw_current, "current").map_err(anyhow::Error::msg)?;
    let previous = raw_previous
        .filter(|previous| previous.get("capture").is_some())
        .map(|previous| snapshot::normalize(&previous, "previous"))
        .transpose()
        .map_err(anyhow::Error::msg)?;
    if let Some(previous) = previous.as_ref() {
        snapshot::same_pr(previous, &current).map_err(anyhow::Error::msg)?;
        snapshot::same_issue(previous, &current).map_err(anyhow::Error::msg)?;
    }

    let mut state = current;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("current PR state must be an object"))?;
    object.insert("reviewControl".into(), control);
    state::check_pr_state(plugin_root, &state, false).map_err(anyhow::Error::msg)?;
    Ok(state)
}

pub(crate) fn import_pre_pr_history(
    _plugin_root: &Path,
    _repository_root: &Path,
    _current_text: &str,
    _envelope_text: &str,
) -> Result<Value> {
    bail!(
        "legacy review-control processing is no longer supported: pre-PR history import is retired"
    )
}

pub(crate) fn produce(
    plugin_root: &Path,
    _repository_root: &Path,
    request_text: &str,
) -> Result<Value> {
    let request: Value = serde_json::from_str(request_text)
        .map_err(|error| anyhow::anyhow!("review control input is invalid: {error}"))?;
    request::reject_retired_inputs(&request).map_err(anyhow::Error::msg)?;

    let control = request
        .get("control_state")
        .or_else(|| request.get("reviewControl"))
        .cloned()
        .unwrap_or_else(|| request.clone());
    request::reject_retired_control(&control).map_err(anyhow::Error::msg)?;
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    let (current_pr_state, previous_pr_state) =
        snapshot::normalize_request_states(&request).map_err(anyhow::Error::msg)?;
    if let Some(current) = current_pr_state {
        if let Some(previous) = previous_pr_state.as_ref()
            && previous.get("capture").is_some()
        {
            snapshot::same_pr(previous, &current).map_err(anyhow::Error::msg)?;
            snapshot::same_issue(previous, &current).map_err(anyhow::Error::msg)?;
        }
        let mut state = current;
        state["reviewControl"] = control.clone();
        state::check_pr_state(plugin_root, &state, false).map_err(anyhow::Error::msg)?;
    } else {
        state::check_control(plugin_root, &control).map_err(anyhow::Error::msg)?;
    }
    Ok(serde_json::json!({"control_state": control}))
}

pub(crate) fn check_next_review_eligibility(
    _plugin_root: &Path,
    _repository_root: &Path,
    _current_text: &str,
    _previous_text: &str,
    _request_text: &str,
) -> Result<Value> {
    bail!(
        "legacy review-control processing is no longer supported: next-review eligibility is retired"
    )
}

pub(crate) fn recover_native_history(
    _plugin_root: &Path,
    _current_text: &str,
    _input_text: &str,
) -> Result<Value> {
    bail!(
        "legacy review-control processing is no longer supported: native review-history recovery is retired"
    )
}
