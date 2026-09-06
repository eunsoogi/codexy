use std::path::Path;

use serde_json::{Map, Value, json};

use super::{history, migration, policy, pre_pr, snapshot};

#[path = "state/check.rs"]
mod check;
#[path = "state/lifecycle.rs"]
mod lifecycle;

pub(super) const CONTROL_SCHEMA: &str = "codexy.review-control-state.v1";

const TERMINAL_RESULTS: [&str; 3] = ["PASS", "BLOCK", "UNOBSERVABLE"];

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
    lifecycle::is_terminal(plugin_root, record)
}

pub(super) fn check_control(plugin_root: &Path, control: &Value) -> Result<(), String> {
    let light = control.get("profile").and_then(Value::as_str) == Some("light");
    let head = control
        .get("reviewed_head")
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
        .or_else(|| light.then_some("light-review"))
        .ok_or_else(|| "review control state must bind reviewed_head".to_owned())?;
    let state = json!({"headRefOid": head, "reviewControl": control});
    if !light {
        control
            .get("issue_number")
            .and_then(Value::as_u64)
            .ok_or_else(|| "review control state must contain numeric issue_number".to_owned())?;
    }
    check::with_mode(
        plugin_root,
        &state,
        false,
        ReviewerMode::Current,
        StateSource::ControlOnly,
    )
}

#[derive(Clone, Copy)]
enum ReviewerMode {
    Current,
    Legacy,
}

#[derive(Clone, Copy)]
enum StateSource {
    ControlOnly,
    PrSnapshot,
}

pub(super) fn check_pr_state(
    plugin_root: &Path,
    state: &Value,
    require_pass: bool,
) -> Result<(), String> {
    check::with_mode(
        plugin_root,
        state,
        require_pass,
        ReviewerMode::Current,
        StateSource::PrSnapshot,
    )
}

pub(super) fn check_pr_state_predecessor(plugin_root: &Path, state: &Value) -> Result<(), String> {
    check::with_mode(
        plugin_root,
        state,
        false,
        ReviewerMode::Legacy,
        StateSource::PrSnapshot,
    )
}

fn count(value: &Map<String, Value>, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("review control state must contain numeric {key}"))
}
