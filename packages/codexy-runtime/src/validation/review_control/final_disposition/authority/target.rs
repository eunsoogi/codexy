use serde_json::{Map, Value};

use super::super::fields::{reject_unknown, text};

pub(super) fn check(
    authority: &Map<String, Value>,
    locator: &Value,
    issue_number: u64,
    expected_repository: Option<&str>,
    expected_pull_request: Option<u64>,
    base: &str,
    current_head: &str,
) -> Result<(), String> {
    let repository = text(authority, "repository", "final disposition authority")?;
    let locator = locator
        .as_object()
        .ok_or("final disposition authority locator must be an object")?;
    if locator.get("repository").and_then(Value::as_str) != Some(repository)
        || locator.get("owningIssue").and_then(Value::as_u64) != Some(issue_number)
        || expected_repository.is_some_and(|expected| repository != expected)
    {
        return Err("final disposition authority repository or issue identity is invalid".into());
    }
    let owning_issue = authority
        .get("owningIssue")
        .and_then(Value::as_object)
        .ok_or("final disposition authority must retain owning issue")?;
    reject_unknown(
        owning_issue,
        &["repository", "number"],
        "final disposition authority owning issue",
    )?;
    if owning_issue.get("repository").and_then(Value::as_str) != Some(repository)
        || owning_issue.get("number").and_then(Value::as_u64) != Some(issue_number)
    {
        return Err("final disposition authority owning issue identity is invalid".into());
    }
    let pull_request = authority
        .get("pullRequest")
        .and_then(Value::as_object)
        .ok_or("final disposition authority must retain pull request")?;
    reject_unknown(
        pull_request,
        &["repository", "number", "baseRefOid", "headRefOid"],
        "final disposition authority pull request",
    )?;
    let pull_number = locator
        .get("pullRequest")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or("final disposition authority locator must contain pullRequest")?;
    if pull_request.get("repository").and_then(Value::as_str) != Some(repository)
        || pull_request.get("number").and_then(Value::as_u64) != Some(pull_number)
        || pull_request.get("baseRefOid").and_then(Value::as_str) != Some(base)
        || pull_request.get("headRefOid").and_then(Value::as_str) != Some(current_head)
        || expected_pull_request.is_some_and(|expected| pull_number != expected)
    {
        return Err("final disposition authority pull request identity is invalid".into());
    }
    Ok(())
}
