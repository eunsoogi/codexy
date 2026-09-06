use serde_json::{Map, Value};

use super::super::pre_pr::text;

pub(super) fn check(
    disposition: &Map<String, Value>,
    current: &Map<String, Value>,
    current_base: &str,
    prior_delta: &Map<String, Value>,
) -> Result<(), String> {
    let current_repository = text(current, "repository", "current")?;
    check_target(disposition, current, current_base, current_repository)?;
    check_maintainer(disposition, prior_delta)
}

fn check_target(
    disposition: &Map<String, Value>,
    current: &Map<String, Value>,
    current_base: &str,
    current_repository: &str,
) -> Result<(), String> {
    if disposition.get("repository").and_then(Value::as_str) != Some(current_repository) {
        return Err("finding disposition changes repository identity".into());
    }
    let issue = disposition
        .get("owningIssue")
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition must bind owning issue".to_owned())?;
    let current_issue = current
        .get("capture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("owningIssue"))
        .and_then(Value::as_object)
        .ok_or_else(|| "current PR snapshot must bind owning issue".to_owned())?;
    if issue.get("repository") != current_issue.get("repository")
        || issue.get("number") != current_issue.get("number")
        || issue.get("url") != current_issue.get("url")
    {
        return Err("finding disposition changes owning issue identity".into());
    }
    let pull = disposition
        .get("pullRequest")
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition must bind pull request".to_owned())?;
    if pull.get("repository").and_then(Value::as_str) != Some(current_repository)
        || pull.get("number") != current.get("number")
        || pull.get("baseRefOid").and_then(Value::as_str) != Some(current_base)
        || pull.get("headRefOid") != current.get("headRefOid")
    {
        return Err("finding disposition changes pull request, base, or head identity".into());
    }
    Ok(())
}

fn check_maintainer(
    disposition: &Map<String, Value>,
    prior_delta: &Map<String, Value>,
) -> Result<(), String> {
    let decision = disposition
        .get("sources")
        .and_then(Value::as_object)
        .and_then(|sources| sources.get("maintainerDecision"))
        .and_then(Value::as_object)
        .and_then(|source| source.get("decision"))
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition must bind maintainer decision facts".to_owned())?;
    if decision.get("accepted") != Some(&Value::Bool(true)) {
        return Err("maintainer policy disposition must be explicitly accepted".into());
    }
    let prior_reviewer = prior_delta
        .get("reviewer")
        .and_then(Value::as_object)
        .ok_or_else(|| "prior delta event must bind reviewer facts".to_owned())?;
    for (decision_key, reviewer_key) in [
        ("reviewer", "name"),
        ("actualModel", "model"),
        ("actualReasoningEffort", "reasoning_effort"),
    ] {
        if text(decision, decision_key, "maintainer decision")?
            != text(prior_reviewer, reviewer_key, "prior delta reviewer")?
        {
            return Err("maintainer decision does not bind the prior delta reviewer tuple".into());
        }
    }
    let finding_id = text(decision, "findingId", "maintainer decision")?;
    let finding_path = text(decision, "path", "maintainer decision")?;
    if !prior_delta
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .is_some_and(|findings| {
            findings.iter().any(|finding| {
                finding.get("id").and_then(Value::as_str) == Some(finding_id)
                    && finding.get("path").and_then(Value::as_str) == Some(finding_path)
                    && finding.get("kind").and_then(Value::as_str) == Some("policy_difference")
            })
        })
    {
        return Err("maintainer decision does not bind a policy-difference finding".into());
    }
    Ok(())
}
