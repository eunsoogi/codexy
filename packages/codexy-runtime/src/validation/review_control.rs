use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

mod classification;
mod external_finding;
mod history;
mod migration;
mod policy;
mod post_cap_disposition;
mod pre_pr;
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
    let mut state = state.clone();
    if let Some(mut control) = state.get("reviewControl").cloned() {
        if external_finding::requires_source(&control) {
            if let Err(error) = external_finding::refresh_live(&mut control) {
                return vec![error];
            }
        } else if post_cap_disposition::requires_source(&control) {
            if let Err(error) = post_cap_disposition::refresh_live(&mut control, Some(&state)) {
                return vec![error];
            }
        }
        if let Some(object) = state.as_object_mut() {
            object.insert("reviewControl".into(), control);
        }
    }
    state::check_pr_state(plugin_root, &state, true)
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
    let mut control: Value = serde_json::from_str(control_text)?;
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    if external_finding::requires_source(&control) {
        external_finding::refresh_live(&mut control).map_err(anyhow::Error::msg)?;
    }
    if post_cap_disposition::requires_source(&control) {
        post_cap_disposition::refresh_live(&mut control, Some(&current))
            .map_err(anyhow::Error::msg)?;
    }
    let previous: Value = serde_json::from_str(previous_text)
        .map_err(|error| anyhow::anyhow!("previous PR state is invalid: {error}"))?;
    let control = if control.get("profile").and_then(Value::as_str) != Some("light")
        || predecessor_has_pre_pr_history(Some(&previous))
    {
        transition::check_with_repository(
            plugin_root,
            repository_root,
            &previous,
            &current,
            &control,
        )
        .map_err(anyhow::Error::msg)?
    } else {
        control
    };
    let mut state = current;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("current PR state must be an object"))?;
    object.insert("reviewControl".into(), control);
    state::check_pr_state(plugin_root, &state, false).map_err(anyhow::Error::msg)?;
    Ok(state)
}

pub(super) fn import_pre_pr_history(
    plugin_root: &Path,
    repository_root: &Path,
    current_text: &str,
    envelope_text: &str,
) -> Result<Value> {
    let current: Value = serde_json::from_str(current_text)?;
    let envelope: Value = serde_json::from_str(envelope_text)
        .map_err(|error| anyhow::anyhow!("pre-PR history input is invalid: {error}"))?;
    pre_pr::import(plugin_root, repository_root, &current, &envelope).map_err(anyhow::Error::msg)
}

pub(super) fn produce(
    plugin_root: &Path,
    repository_root: &Path,
    request_text: &str,
) -> Result<Value> {
    let request: Value = serde_json::from_str(request_text)
        .map_err(|error| anyhow::anyhow!("review control input is invalid: {error}"))?;
    let mut control = request
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
    if request.get("authenticated_external_finding").is_some()
        || request
            .get("authenticated_external_finding_capture")
            .is_some()
        || request.get("authenticated_finding_disposition").is_some()
        || request
            .get("authenticated_finding_disposition_capture")
            .is_some()
        || request.get("finding_disposition").is_some()
    {
        bail!(
            "review control producer rejects caller-supplied external finding source or capture; provide authenticated_external_finding_locator"
        );
    }
    if let Some(locator) = request.get("authenticated_external_finding_locator") {
        let expected_commit = qualifying_change_from_head(&control).map(ToOwned::to_owned);
        let source = external_finding::read_live(locator, expected_commit.as_deref())
            .map_err(anyhow::Error::msg)?;
        external_finding::normalize_producer(&mut control, &source).map_err(anyhow::Error::msg)?;
    } else if let Some(locator) = request.get("authenticated_finding_disposition_locator") {
        let current = request.get("current_pr_state").ok_or_else(|| {
            anyhow::anyhow!("finding disposition producer requires current_pr_state")
        })?;
        post_cap_disposition::validate_locator(locator, current).map_err(anyhow::Error::msg)?;
        let expected_head = qualifying_change_to_head(&control);
        let source =
            post_cap_disposition::read_live(locator, expected_head).map_err(anyhow::Error::msg)?;
        let previous = request.get("previous_pr_state").ok_or_else(|| {
            anyhow::anyhow!("finding disposition producer requires previous_pr_state")
        })?;
        post_cap_disposition::normalize_producer(&mut control, &source, previous)
            .map_err(anyhow::Error::msg)?;
    } else if external_finding::requires_source(&control) {
        bail!(
            "review control producer requires authenticated_external_finding_locator for external repair"
        );
    } else if post_cap_disposition::requires_source(&control) {
        bail!(
            "review control producer requires authenticated_finding_disposition_locator for mixed findings"
        );
    }
    let control = if control
        .get("profile")
        .and_then(Value::as_str)
        .is_some_and(|profile| profile != "light")
        || predecessor_has_pre_pr_history(request.get("previous_pr_state"))
    {
        let raw_error = state::check_control(plugin_root, &control).err();
        let current = request
            .get("current_pr_state")
            .ok_or_else(|| anyhow::anyhow!("review control producer requires current_pr_state"))?;
        let previous = request
            .get("previous_pr_state")
            .ok_or_else(|| anyhow::anyhow!("review control producer requires previous_pr_state"))?;
        match transition::check_with_repository(
            plugin_root,
            repository_root,
            previous,
            current,
            &control,
        ) {
            Ok(normalized) => normalized,
            Err(error) => match raw_error {
                None => return Err(anyhow::Error::msg(error)),
                Some(raw_error) => return Err(anyhow::Error::msg(raw_error)),
            },
        }
    } else {
        control
    };
    state::check_control(plugin_root, &control).map_err(anyhow::Error::msg)?;
    Ok(serde_json::json!({"control_state": control}))
}

fn predecessor_has_pre_pr_history(state: Option<&Value>) -> bool {
    state
        .and_then(Value::as_object)
        .and_then(|state| state.get("reviewControl"))
        .and_then(Value::as_object)
        .is_some_and(|control| control.contains_key("pre_pr_import"))
}

fn qualifying_change_from_head(control: &Value) -> Option<&str> {
    control
        .get("post_cap_re_review")
        .and_then(Value::as_object)
        .and_then(|post_cap| post_cap.get("qualifying_change"))
        .and_then(Value::as_object)
        .and_then(|change| change.get("from_head"))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
}

fn qualifying_change_to_head(control: &Value) -> Option<&str> {
    control
        .get("post_cap_re_review")
        .and_then(Value::as_object)
        .and_then(|post_cap| post_cap.get("qualifying_change"))
        .and_then(Value::as_object)
        .and_then(|change| change.get("to_head"))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
}
