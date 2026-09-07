use serde_json::{Map, Value, json};

use super::super::Locator;
use crate::validation::review_control::pre_pr::{number, object, reject_unknown, text};

pub(super) fn check_shape(raw: Option<&Value>) -> Result<(), String> {
    let raw = object(raw, "Actions source ownership")?;
    reject_unknown(raw, &["response", "projection"], "Actions source ownership")?;
    object(raw.get("response"), "Actions source ownership response")?;
    let projection = object(raw.get("projection"), "Actions source ownership projection")?;
    reject_unknown(
        projection,
        &["repository", "pullRequest", "owningIssue"],
        "Actions source ownership projection",
    )
}

pub(super) fn project(response: &Value, locator: &Locator) -> Result<Map<String, Value>, String> {
    let root = object(Some(response), "Actions source ownership response")?;
    if root
        .get("errors")
        .is_some_and(|errors| errors.as_array().is_none_or(|items| !items.is_empty()))
    {
        return Err("Actions source ownership response contains errors".into());
    }
    let data = object(root.get("data"), "Actions source ownership response data")?;
    let repository = object(
        data.get("repository"),
        "Actions source ownership repository",
    )?;
    let pull = object(
        repository.get("pullRequest"),
        "Actions source ownership pull request",
    )?;
    reject_unknown(
        pull,
        &["number", "url", "repository", "closingIssuesReferences"],
        "Actions source ownership pull request",
    )?;
    let pull_repository = object(
        pull.get("repository"),
        "Actions source ownership pull request repository",
    )?;
    reject_unknown(
        pull_repository,
        &["nameWithOwner"],
        "Actions source ownership pull request repository",
    )?;
    if number(pull, "number", "Actions source ownership pull request")? != locator.pull_request
        || text(pull, "url", "Actions source ownership pull request")?
            != format!(
                "https://github.com/{}/pull/{}",
                locator.repository, locator.pull_request
            )
        || text(
            pull_repository,
            "nameWithOwner",
            "Actions source ownership pull request repository",
        )? != locator.repository
    {
        return Err("Actions source pull request identity does not match locator".into());
    }
    let closing = object(
        pull.get("closingIssuesReferences"),
        "Actions source ownership closing issues",
    )?;
    reject_unknown(
        closing,
        &["nodes", "pageInfo"],
        "Actions source ownership closing issues",
    )?;
    let page_info = object(
        closing.get("pageInfo"),
        "Actions source ownership closing issues page info",
    )?;
    reject_unknown(
        page_info,
        &["hasNextPage"],
        "Actions source ownership closing issues page info",
    )?;
    if page_info.get("hasNextPage") != Some(&Value::Bool(false)) {
        return Err("Actions source ownership closing issues are incomplete".into());
    }
    let nodes = closing
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "Actions source ownership closing issues nodes are invalid".to_owned())?;
    let matches = nodes
        .iter()
        .filter_map(Value::as_object)
        .filter(|issue| issue.get("number").and_then(Value::as_u64) == Some(locator.owning_issue))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("Actions source PR does not identify one authenticated owning issue".into());
    }
    let issue = matches[0];
    reject_unknown(
        issue,
        &["number", "url", "repository"],
        "Actions source ownership issue",
    )?;
    let issue_repository = object(
        issue.get("repository"),
        "Actions source ownership issue repository",
    )?;
    reject_unknown(
        issue_repository,
        &["nameWithOwner"],
        "Actions source ownership issue repository",
    )?;
    if text(issue, "url", "Actions source ownership issue")?
        != format!(
            "https://github.com/{}/issues/{}",
            locator.repository, locator.owning_issue
        )
        || text(
            issue_repository,
            "nameWithOwner",
            "Actions source ownership issue repository",
        )? != locator.repository
    {
        return Err("Actions source owning issue identity does not match locator".into());
    }
    Ok(Map::from_iter([
        (
            "repository".into(),
            Value::String(locator.repository.clone()),
        ),
        (
            "pullRequest".into(),
            json!({
                "repository": locator.repository,
                "number": locator.pull_request,
                "url": format!("https://github.com/{}/pull/{}", locator.repository, locator.pull_request)
            }),
        ),
        (
            "owningIssue".into(),
            json!({
                "repository": locator.repository,
                "number": locator.owning_issue,
                "url": format!("https://github.com/{}/issues/{}", locator.repository, locator.owning_issue),
                "association": "closing-issue-reference"
            }),
        ),
    ]))
}

pub(super) fn authenticated_projection(
    raw: Option<&Value>,
    locator: &Locator,
) -> Result<Map<String, Value>, String> {
    check_shape(raw)?;
    let raw = object(raw, "Actions source ownership")?;
    let response = raw
        .get("response")
        .ok_or_else(|| "Actions source ownership response is missing".to_owned())?;
    let expected = object(raw.get("projection"), "Actions source ownership projection")?;
    let actual = project(response, locator)?;
    if expected != &actual {
        return Err("Actions source ownership projection does not match live response".into());
    }
    Ok(actual)
}
