use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

mod classification;
mod operations;
mod policy;
mod request;
mod snapshot;
mod state;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    policy::load(plugin_root)
        .and_then(|_| classification::check(plugin_root))
        .map_or_else(|error| vec![error.to_string()], |_| Vec::new())
}

pub(super) fn resolve_profile(plugin_root: &Path, request: &str) -> Result<Value> {
    policy::resolve(plugin_root, request)
}

pub(super) use operations::{
    build_pr_state, check_next_review_eligibility, import_pre_pr_history, produce,
    recover_native_history,
};

pub(super) fn check_packet(
    _plugin_root: &Path,
    _repository_root: &Path,
    _legacy_output: &Path,
    _legacy_input: &str,
) -> Result<()> {
    bail!(
        "legacy review-control processing is no longer supported: review packet validation is retired"
    )
}

pub(super) fn check_economics(
    _plugin_root: &Path,
    _repository_root: &Path,
    _legacy_input: &str,
) -> Result<()> {
    bail!(
        "legacy review-control processing is no longer supported: review economics validation is retired"
    )
}

pub(super) fn reject_retired_inputs(state: &Value) -> Result<(), String> {
    request::reject_retired_inputs(state)
}

pub(super) fn check_handoff(plugin_root: &Path, state: &Value) -> Vec<String> {
    if let Err(error) = request::reject_retired_inputs(state) {
        return vec![error];
    }
    state::check_pr_state(plugin_root, state, true)
        .err()
        .into_iter()
        .collect()
}

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
    state::is_lifecycle_terminal(plugin_root, record)
}

pub(super) fn lifecycle_error(plugin_root: &Path, record: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(record).ok()?;
    if let Err(error) = request::reject_retired_inputs(&value) {
        return Some(error);
    }
    if value.get("control_state").is_none()
        && value.get("reviewControl").is_none()
        && let Err(error) = request::reject_retired_control(&value)
    {
        return Some(error);
    }
    let object = value.as_object()?;
    if object.contains_key("reviewed_head")
        && object.contains_key("profile")
        && object.contains_key("reviewer")
        && !state::is_lifecycle_terminal(plugin_root, record)
        && !state::is_lifecycle_pending(plugin_root, record)
    {
        return Some("review lifecycle evidence must contain direct terminal fields".to_owned());
    }
    None
}
