use std::path::Path;

use serde_json::Value;

use super::request;

#[path = "simple.rs"]
mod simple;

pub(super) const CONTROL_SCHEMA: &str = "codexy.review-control-state.v1";

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(record) else {
        return false;
    };
    request::reject_retired_control(&value).is_ok() && simple::is_terminal(plugin_root, &value)
}

pub(super) fn is_lifecycle_pending(plugin_root: &Path, record: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(record) else {
        return false;
    };
    request::reject_retired_control(&value).is_ok() && simple::is_pending(plugin_root, &value)
}

pub(super) fn check_control(plugin_root: &Path, control: &Value) -> Result<(), String> {
    request::reject_retired_inputs(control)?;
    simple::check_control(plugin_root, control)
}

pub(super) fn check_pr_state(
    plugin_root: &Path,
    state: &Value,
    require_pass: bool,
) -> Result<(), String> {
    request::reject_retired_inputs(state)?;
    simple::check_pr_state(plugin_root, state, require_pass)
}
