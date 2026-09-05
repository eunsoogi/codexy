use std::path::Path;

use serde_json::{Map, Value, json};

use super::{history, migration, policy, snapshot};

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
    check_with_mode(
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
    check_with_mode(
        plugin_root,
        state,
        require_pass,
        ReviewerMode::Current,
        StateSource::PrSnapshot,
    )
}

pub(super) fn check_pr_state_predecessor(plugin_root: &Path, state: &Value) -> Result<(), String> {
    check_with_mode(
        plugin_root,
        state,
        false,
        ReviewerMode::Legacy,
        StateSource::PrSnapshot,
    )
}

fn check_with_mode(
    plugin_root: &Path,
    state: &Value,
    require_pass: bool,
    reviewer_mode: ReviewerMode,
    source: StateSource,
) -> Result<(), String> {
    let head = state
        .get("headRefOid")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
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

    if matches!(source, StateSource::PrSnapshot) {
        snapshot::check(state, "current")?;
    }

    if profile.reviewer.is_none() {
        if matches!(reviewer_mode, ReviewerMode::Legacy) {
            return Err("light review selection cannot have a legacy reviewer".into());
        }
        if control
            .get("reviewer")
            .is_some_and(|reviewer| !reviewer.is_null())
        {
            return Err("light review selection must not attach a reviewer".into());
        }
        if [
            "reviewed_head",
            "terminal_result",
            "unresolved_findings",
            "full_review_count",
            "delta_review_count",
            "issue_number",
            "terminal_review_count",
            "terminal_review_limit",
            "terminal_review_history",
            "post_cap_re_review",
            "reviewer_migration",
        ]
        .iter()
        .any(|field| control.contains_key(*field))
        {
            return Err("light review selection must not carry terminal review state".into());
        }
        return Ok(());
    }

    let issue_number = count(control, "issue_number")?;
    let Some(reviewer) = profile.reviewer.as_ref() else {
        return Err("selected reviewer is unavailable".into());
    };
    if matches!(source, StateSource::PrSnapshot)
        && snapshot::owning_issue_number(state, "current")? != issue_number
    {
        return Err("review control state issue_number disagrees with the owning issue".into());
    }
    let current_reviewer = serde_json::to_value(reviewer)
        .map_err(|_| "selected reviewer is not serializable".to_owned())?;
    let history_len = control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let migration_boundary = match reviewer_mode {
        ReviewerMode::Current => {
            migration::boundary(control, selected, &current_reviewer, history_len)?
        }
        ReviewerMode::Legacy => {
            if control.contains_key("reviewer_migration") {
                return Err("legacy predecessor state must not carry reviewer migration".into());
            }
            None
        }
    };
    let legacy_reviewer = policy::legacy_reviewer(selected);
    let expected_reviewer = match reviewer_mode {
        ReviewerMode::Current => &current_reviewer,
        ReviewerMode::Legacy => match legacy_reviewer.as_ref() {
            Some(reviewer) => reviewer,
            None => return Err("legacy reviewer must be available".into()),
        },
    };
    if control.get("reviewer") != Some(expected_reviewer) {
        return Err("review control state does not bind the selected reviewer".into());
    }
    let reviewed_head = control
        .get("reviewed_head")
        .or_else(|| control.get("head_oid"))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
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
    let terminal_count = count(control, "terminal_review_count")?;
    let terminal_limit = count(control, "terminal_review_limit")?;
    if terminal_limit != u64::from(profile.terminal_review_limit) {
        return Err("review control state terminal review limit disagrees with policy".into());
    }
    history::check(
        control,
        &history::CheckContext {
            expected_reviewer,
            legacy_reviewer: migration_boundary.and(legacy_reviewer.as_ref()),
            legacy_history_boundary: migration_boundary,
            reviewed_head,
            terminal,
            findings,
            full_count,
            delta_count,
            terminal_count,
            terminal_limit,
            profile,
        },
    )?;
    if require_pass && terminal != "PASS" {
        return Err("review control state terminal_result is not PASS".into());
    }
    if require_pass && !findings.is_empty() {
        return Err("review control state has unresolved actionable findings".into());
    }
    Ok(())
}

fn count(value: &Map<String, Value>, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("review control state must contain numeric {key}"))
}
