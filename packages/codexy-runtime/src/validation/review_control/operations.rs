use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

use super::{
    external_finding, final_disposition, post_cap_disposition, pre_pr, pre_verdict, request,
    snapshot, state, transition,
};

pub(crate) fn build_pr_state(
    plugin_root: &Path,
    repository_root: &Path,
    current_text: &str,
    control_text: &str,
    previous_text: &str,
) -> Result<Value> {
    let mut control: Value = serde_json::from_str(control_text)?;
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    let simple = state::is_simple_control(&control);
    let current: Value = snapshot::normalize(&serde_json::from_str(current_text)?, "current")
        .map_err(anyhow::Error::msg)?;
    if external_finding::requires_source(&control) {
        external_finding::refresh_live(&mut control).map_err(anyhow::Error::msg)?;
    }
    if post_cap_disposition::requires_source(&control) {
        post_cap_disposition::refresh_live(&mut control, Some(&current))
            .map_err(anyhow::Error::msg)?;
    }
    if control.get("final_disposition").is_some() {
        final_disposition::refresh_live(&mut control, None, &current)
            .map_err(anyhow::Error::msg)?;
    }
    let previous_has_pre_pr_history = serde_json::from_str::<Value>(previous_text)
        .ok()
        .as_ref()
        .is_some_and(|previous| request::predecessor_has_pre_pr_history(Some(previous)));
    let control = if simple && !previous_has_pre_pr_history {
        control
    } else {
        let previous = snapshot::normalize(
            &serde_json::from_str(previous_text)
                .map_err(|error| anyhow::anyhow!("previous PR state is invalid: {error}"))?,
            "previous",
        )
        .map_err(anyhow::Error::msg)?;
        if control.get("profile").and_then(Value::as_str) != Some("light")
            || previous_has_pre_pr_history
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
        }
    };
    let mut state = current;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("current PR state must be an object"))?;
    object.insert("reviewControl".into(), control);
    state::check_pr_state(plugin_root, &state, false).map_err(anyhow::Error::msg)?;
    Ok(state)
}

pub(crate) fn import_pre_pr_history(
    plugin_root: &Path,
    repository_root: &Path,
    current_text: &str,
    envelope_text: &str,
) -> Result<Value> {
    let current = snapshot::normalize(&serde_json::from_str(current_text)?, "current")
        .map_err(anyhow::Error::msg)?;
    let envelope: Value = serde_json::from_str(envelope_text)
        .map_err(|error| anyhow::anyhow!("pre-PR history input is invalid: {error}"))?;
    pre_pr::import(plugin_root, repository_root, &current, &envelope).map_err(anyhow::Error::msg)
}

pub(crate) fn produce(
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
    let (current_pr_state, previous_pr_state) =
        snapshot::normalize_request_states(&request).map_err(anyhow::Error::msg)?;
    if let Some(disposition) = control.get("final_disposition") {
        if disposition
            .as_object()
            .is_some_and(|object| object.contains_key("authority"))
        {
            bail!(
                "review control producer rejects caller-supplied final disposition authority; provide authenticated_final_disposition_locator"
            );
        }
        let current = current_pr_state.as_ref().ok_or_else(|| {
            anyhow::anyhow!("final disposition producer requires current_pr_state")
        })?;
        let locator = request
            .get("authenticated_final_disposition_locator")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "final disposition producer requires authenticated_final_disposition_locator"
                )
            })?;
        final_disposition::refresh_live(&mut control, Some(locator), current)
            .map_err(anyhow::Error::msg)?;
    } else if request
        .get("authenticated_final_disposition_locator")
        .is_some()
    {
        bail!("authenticated_final_disposition_locator requires final_disposition");
    }
    if request::has_caller_supplied_finding(&request) {
        bail!(
            "review control producer rejects caller-supplied external finding source or capture; provide authenticated_external_finding_locator"
        );
    }
    let graphql_locator = request.get("authenticated_external_finding_locator");
    let actions_locator = request.get("authenticated_actions_finding_locator");
    let expected_commit = request::qualifying_change_from_head(&control).map(ToOwned::to_owned);
    let source = external_finding::read_from_locators(
        graphql_locator,
        actions_locator,
        expected_commit.as_deref(),
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(source) = source {
        external_finding::normalize_producer(&mut control, &source).map_err(anyhow::Error::msg)?;
    } else if let Some(locator) = request.get("authenticated_finding_disposition_locator") {
        let current = current_pr_state.as_ref().ok_or_else(|| {
            anyhow::anyhow!("finding disposition producer requires current_pr_state")
        })?;
        post_cap_disposition::validate_locator(locator, current).map_err(anyhow::Error::msg)?;
        let expected_head = request::qualifying_change_to_head(&control);
        let source =
            post_cap_disposition::read_live(locator, expected_head).map_err(anyhow::Error::msg)?;
        let previous = previous_pr_state.as_ref().ok_or_else(|| {
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
    let simple = state::is_simple_control(&control);
    let previous_has_pre_pr_history =
        request::predecessor_has_pre_pr_history(previous_pr_state.as_ref());
    let control = if previous_has_pre_pr_history
        || (!simple
            && control
                .get("profile")
                .and_then(Value::as_str)
                .is_some_and(|profile| profile != "light"))
    {
        let raw_error = state::check_control(plugin_root, &control).err();
        let current = current_pr_state
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("review control producer requires current_pr_state"))?;
        let previous = previous_pr_state
            .as_ref()
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
    if simple && let Some(current) = current_pr_state {
        let mut state = current;
        state["reviewControl"] = control.clone();
        state::check_pr_state(plugin_root, &state, false).map_err(anyhow::Error::msg)?;
    }
    state::check_control(plugin_root, &control).map_err(anyhow::Error::msg)?;
    Ok(serde_json::json!({"control_state": control}))
}

pub(crate) fn check_next_review_eligibility(
    plugin_root: &Path,
    repository_root: &Path,
    current_text: &str,
    previous_text: &str,
    request_text: &str,
) -> Result<Value> {
    pre_verdict::check(
        plugin_root,
        repository_root,
        current_text,
        previous_text,
        request_text,
    )
    .map_err(anyhow::Error::msg)
}
