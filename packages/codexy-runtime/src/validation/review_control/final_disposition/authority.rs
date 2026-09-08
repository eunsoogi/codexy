use serde_json::{Map, Value};

mod body;
mod check;
mod refresh;
mod source;
mod target;
mod threads;

use super::fields::{finding_ids, text};

pub(super) const SCHEMA: &str = "codexy.review-control-final-authority.v1";
const CAPTURE_METHOD: &str = "graphql+gh-pr-view+github-api";

pub(super) fn refresh(
    control: &mut Value,
    locator: Option<&Value>,
    current: &Value,
) -> Result<(), String> {
    refresh::refresh(control, locator, current)
}

pub(super) fn check(
    disposition: &Map<String, Value>,
    history: &[Value],
    source_head: &str,
    current_head: &str,
    base: &str,
    issue_number: u64,
    expected_repository: Option<&str>,
    expected_pull_request: Option<u64>,
    pr_state: Option<&Value>,
) -> Result<(), String> {
    let third = history
        .get(2)
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition authority requires a third review event".to_owned())?;
    let third_findings = third
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "final disposition authority requires third findings".to_owned())?;
    let expected_ids = finding_ids(third_findings, "third-review findings")?;
    let authority = disposition
        .get("authority")
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition requires authenticated authority".to_owned())?;
    super::fields::reject_unknown(
        authority,
        &[
            "schema",
            "locator",
            "repository",
            "owningIssue",
            "pullRequest",
            "sources",
            "capture",
        ],
        "final disposition authority",
    )?;
    if text(authority, "schema", "final disposition authority")? != SCHEMA {
        return Err("final disposition authority has an unsupported schema".into());
    }
    let locator = authority
        .get("locator")
        .ok_or_else(|| "final disposition authority must retain locator".to_owned())?;
    super::super::post_cap_disposition::check_locator(locator)?;
    target::check(
        authority,
        locator,
        issue_number,
        expected_repository,
        expected_pull_request,
        base,
        current_head,
    )?;
    let sources = authority
        .get("sources")
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition authority must retain sources".to_owned())?;
    super::fields::reject_unknown(
        sources,
        &["currentHeadCi", "maintainerDecision"],
        "final disposition authority sources",
    )?;
    let pull_number = authority
        .get("pullRequest")
        .and_then(Value::as_object)
        .and_then(|pull| pull.get("number"))
        .and_then(Value::as_u64)
        .ok_or("final disposition authority pull request must contain number")?;
    check::check_ci(
        sources
            .get("currentHeadCi")
            .ok_or_else(|| "final disposition authority must retain current-head CI".to_owned())?,
        authority,
        base,
        current_head,
        pull_number,
    )?;
    check::check_maintainer(
        sources.get("maintainerDecision").ok_or_else(|| {
            "final disposition authority must retain maintainer decision".to_owned()
        })?,
        authority,
        source_head,
        current_head,
        base,
        text(disposition, "review_event_id", "final disposition")?,
        issue_number,
        pull_number,
        &expected_ids,
    )?;
    if let Some(pr_state) = pr_state {
        check_state_binding(
            pr_state,
            sources.get("maintainerDecision").ok_or_else(|| {
                "final disposition authority must retain maintainer decision".to_owned()
            })?,
        )?;
    }
    let capture = authority
        .get("capture")
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition authority must retain capture".to_owned())?;
    super::fields::reject_unknown(
        capture,
        &["provider", "method", "authenticated"],
        "final disposition authority capture",
    )?;
    if text(capture, "provider", "final disposition authority capture")? != "github"
        || text(capture, "method", "final disposition authority capture")? != CAPTURE_METHOD
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err("final disposition authority is not an authenticated GitHub source".into());
    }
    Ok(())
}

fn check_state_binding(state: &Value, source: &Value) -> Result<(), String> {
    check::check_pr_state_snapshot(state)?;
    let source = source
        .as_object()
        .ok_or("final disposition maintainer decision must be an object")?;
    let live_state = source
        .get("prState")
        .ok_or("final disposition maintainer decision must retain prState")?;
    check_state_values(state, live_state)
}

fn check_state_values(state: &Value, live_state: &Value) -> Result<(), String> {
    for key in ["state", "isDraft", "mergeStateStatus", "reviewThreads"] {
        if state.get(key) != live_state.get(key) {
            return Err(format!(
                "final disposition PR state disagrees with authenticated live {key}"
            ));
        }
    }
    Ok(())
}

pub(super) fn check_handoff_state(
    state_value: &Value,
    control: &Map<String, Value>,
) -> Result<(), String> {
    check::check_pr_state_snapshot(state_value)?;
    let authority = control
        .get("final_disposition")
        .and_then(Value::as_object)
        .and_then(|disposition| disposition.get("authority"))
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition handoff requires authority".to_owned())?;
    let source = authority
        .get("sources")
        .and_then(Value::as_object)
        .and_then(|sources| sources.get("maintainerDecision"))
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition handoff requires live PR state".to_owned())?;
    let live_state = source
        .get("prState")
        .ok_or_else(|| "final disposition handoff requires live PR state".to_owned())?;
    check_state_values(state_value, live_state)?;
    Ok(())
}
