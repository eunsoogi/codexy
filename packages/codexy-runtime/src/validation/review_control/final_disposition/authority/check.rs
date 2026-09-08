use serde_json::{Map, Value};

use super::super::super::pre_pr::object;
use super::super::fields::text;
use crate::validation::review_thread_evidence;

const CI_SCHEMA: &str = "codexy.github-current-head-ci.v1";

mod maintainer;

pub(super) use maintainer::check_maintainer;

pub(super) fn check_ci(
    value: &Value,
    authority: &Map<String, Value>,
    base: &str,
    current_head: &str,
    pull_request: u64,
) -> Result<(), String> {
    let ci = object(Some(value), "final disposition current-head CI")?;
    if text(ci, "schema", "final disposition current-head CI")? != CI_SCHEMA
        || text(ci, "repository", "final disposition current-head CI")?
            != text(authority, "repository", "final disposition authority")?
        || ci.get("complete") != Some(&Value::Bool(true))
        || ci.get("pullRequest").and_then(Value::as_u64) != Some(pull_request)
        || text(ci, "headRefOid", "final disposition current-head CI")? != current_head
        || text(ci, "baseRefOid", "final disposition current-head CI")? != base
    {
        return Err("final disposition authority CI is not complete for the exact head".into());
    }
    for key in [
        "checks",
        "requiredStatusChecks",
        "expectedCheckRuns",
        "checkSuites",
    ] {
        if !ci.contains_key(key) {
            return Err(format!("final disposition authority CI is missing {key}"));
        }
    }
    Ok(())
}

pub(super) fn check_pr_state_snapshot(value: &Value) -> Result<(), String> {
    let state = object(Some(value), "final disposition handoff PR state")?;
    if text(state, "state", "final disposition handoff PR state")? != "OPEN"
        || state.get("isDraft") != Some(&Value::Bool(false))
        || text(
            state,
            "mergeStateStatus",
            "final disposition handoff PR state",
        )? != "CLEAN"
    {
        return Err("final disposition handoff PR state is not open, ready, and clean".into());
    }
    let threads = state
        .get("reviewThreads")
        .ok_or("final disposition handoff PR state must retain reviewThreads")?;
    if let Some(error) = review_thread_evidence::check(threads) {
        return Err(error);
    }
    let nodes = threads
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("final disposition handoff PR state must list review thread nodes")?;
    if nodes
        .iter()
        .any(|thread| thread.get("isResolved") != Some(&Value::Bool(true)))
    {
        return Err("final disposition handoff PR state has unresolved review threads".into());
    }
    Ok(())
}
