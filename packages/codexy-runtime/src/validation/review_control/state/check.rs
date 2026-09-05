use std::path::Path;

use serde_json::Value;

use super::{
    CONTROL_SCHEMA, ReviewerMode, StateSource, TERMINAL_RESULTS, count, history, migration, policy,
    pre_pr, snapshot,
};

pub(super) fn with_mode(
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
            "pre_pr_import",
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
    let migration_mode = match reviewer_mode {
        ReviewerMode::Current => {
            migration::mode(control, selected, &current_reviewer, history_len)?
        }
        ReviewerMode::Legacy => {
            if control.contains_key("reviewer_migration") {
                return Err("legacy predecessor state must not carry reviewer migration".into());
            }
            migration::HistoryMode::None
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
    let (legacy_reviewer, legacy_history_boundary, legacy_history_event) = match migration_mode {
        migration::HistoryMode::None => (None, None, None),
        migration::HistoryMode::LegacyPrefix(boundary) => {
            (legacy_reviewer.as_ref(), Some(boundary), None)
        }
        migration::HistoryMode::LegacyEvent(index) => (legacy_reviewer.as_ref(), None, Some(index)),
    };
    history::check(
        control,
        &history::CheckContext {
            expected_reviewer,
            legacy_reviewer,
            legacy_history_boundary,
            legacy_history_event,
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
    pre_pr::check_state(
        matches!(source, StateSource::PrSnapshot),
        require_pass,
        head,
        reviewed_head,
        control,
    )?;
    Ok(())
}
