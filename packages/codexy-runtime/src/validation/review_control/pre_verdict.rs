use std::path::Path;

use serde_json::{Value, json};

use super::{post_cap_disposition, pre_pr, snapshot, state, transition};

const SCHEMA: &str = "codexy.review-control-next-review-eligibility.v1";

pub(super) fn check(
    plugin_root: &Path,
    repository_root: &Path,
    current_text: &str,
    previous_text: &str,
    request_text: &str,
) -> Result<Value, String> {
    let current: Value = serde_json::from_str(current_text)
        .map_err(|error| format!("current PR state is invalid: {error}"))?;
    let previous: Value = serde_json::from_str(previous_text)
        .map_err(|error| format!("previous PR state is invalid: {error}"))?;
    let request: Value = serde_json::from_str(request_text)
        .map_err(|error| format!("next-review eligibility input is invalid: {error}"))?;
    let request = pre_pr::object(Some(&request), "next-review eligibility input")?;
    pre_pr::reject_unknown(
        request,
        &["authenticated_finding_disposition_locator"],
        "next-review eligibility input",
    )?;
    let locator = request
        .get("authenticated_finding_disposition_locator")
        .ok_or_else(|| {
            "next-review eligibility requires a finding disposition locator".to_owned()
        })?;

    snapshot::check(&previous, "previous")?;
    snapshot::check(&current, "current")?;
    if current.get("reviewControl").is_some() {
        return Err("current PR snapshot must not carry reviewControl".into());
    }
    snapshot::same_pr(&previous, &current)?;
    snapshot::same_issue(&previous, &current)?;
    state::check_pr_state(plugin_root, &previous, false)?;
    let previous_control =
        pre_pr::object(previous.get("reviewControl"), "previous review control")?;
    let history = previous_control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .ok_or_else(|| "previous review control must carry terminal history".to_owned())?;
    if previous_control.get("terminal_review_count") != Some(&json!(2))
        || history.len() != 2
        || previous_control.get("post_cap_re_review").is_some()
    {
        return Err("next-review eligibility requires an exact two-event predecessor".into());
    }
    let prior_delta = pre_pr::object(history.get(1), "prior delta event")?;
    let previous_object = previous.as_object().ok_or("previous PR snapshot")?;
    let current_object = current.as_object().ok_or("current PR snapshot")?;
    let previous_head = snapshot::required_oid(previous_object, "headRefOid", "previous")?;
    let previous_base = snapshot::required_oid(previous_object, "baseRefOid", "previous")?;
    let current_base = snapshot::required_oid(current_object, "baseRefOid", "current")?;
    let current_head = snapshot::required_oid(current_object, "headRefOid", "current")?;
    if previous_base != current_base {
        return Err("next-review eligibility must preserve baseRefOid".into());
    }
    if previous_head == current_head {
        return Err("next-review eligibility requires a changed current head".into());
    }
    pre_pr::check_ancestor(
        repository_root,
        previous_head,
        current_head,
        "next-review eligibility",
    )?;
    post_cap_disposition::validate_locator(locator, &current)?;
    let source = post_cap_disposition::read_live(locator, Some(current_head))?;
    let (source, finding_ids) = post_cap_disposition::derive(&source, prior_delta)?;
    transition::check_pre_verdict(&transition::PreVerdictContext {
        repository_root,
        previous_base,
        current_base,
        current: current_object,
        prior_delta,
        from: previous_head,
        to: current_head,
        source: &source,
    })?;
    let findings = source
        .get("findings")
        .cloned()
        .ok_or_else(|| "next-review eligibility source lacks finding coverage".to_owned())?;
    Ok(json!({
        "schema": SCHEMA,
        "eligible": true,
        "target": {
            "repository": current["repository"],
            "owningIssue": snapshot::owning_issue_number(&current, "current")?,
            "pullRequest": current["number"],
            "baseRefOid": current_base,
            "headRefOid": current_head
        },
        "predecessor": {
            "terminalReviewCount": 2,
            "delta": {
                "reviewedHead": previous_head,
                "terminalResult": "BLOCK",
                "findingIds": finding_ids
            }
        },
        "evidence": {
            "kind": post_cap_disposition::REASON,
            "locator": source["locator"],
            "currentHeadCi": source["sources"]["currentHeadCi"],
            "maintainerDecision": source["sources"]["maintainerDecision"],
            "evidenceCommit": current_head
        },
        "coverage": findings
    }))
}
