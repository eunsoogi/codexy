use serde_json::{Map, Value};

use super::super::super::super::pre_pr::{number, object, text};
use super::super::live::Locator;

pub(super) fn check_pull_identity(
    pull: &Map<String, Value>,
    locator: &Locator,
    label: &str,
) -> Result<(), String> {
    let repository = object(pull.get("repository"), &format!("{label} repository"))?;
    if number(pull, "number", label)? != locator.pull_request
        || text(repository, "nameWithOwner", &format!("{label} repository"))? != locator.repository
        || text(pull, "url", label)?
            != format!(
                "https://github.com/{}/pull/{}",
                locator.repository, locator.pull_request
            )
    {
        return Err(format!(
            "authenticated GitHub {label} identity does not match locator"
        ));
    }
    Ok(())
}

pub(super) fn check_issue_identity(
    issue: &Map<String, Value>,
    locator: &Locator,
) -> Result<(), String> {
    if number(issue, "number", "authenticated GitHub owning issue")? != locator.owning_issue
        || text(issue, "url", "authenticated GitHub owning issue")?
            != format!(
                "https://github.com/{}/issues/{}",
                locator.repository, locator.owning_issue
            )
        || text(
            object(
                issue.get("repository"),
                "authenticated GitHub owning issue repository",
            )?,
            "nameWithOwner",
            "authenticated GitHub owning issue repository",
        )? != locator.repository
    {
        return Err("authenticated GitHub owning issue identity does not match locator".into());
    }
    Ok(())
}

pub(super) fn check_comment_identity(
    comment: &Map<String, Value>,
    locator: &Locator,
    pull: &Map<String, Value>,
) -> Result<(), String> {
    if text(comment, "id", "authenticated GitHub review comment")? != locator.review_comment {
        return Err("authenticated GitHub review comment does not match locator".into());
    }
    let comment_pull = object(
        comment.get("pullRequest"),
        "authenticated GitHub review comment pull request",
    )?;
    if comment_pull != pull {
        check_pull_identity(comment_pull, locator, "review comment pull request")?;
    }
    Ok(())
}

pub(super) fn complete(connection: &Map<String, Value>, label: &str) -> Result<(), String> {
    if object(connection.get("pageInfo"), &format!("{label} page info"))?.get("hasNextPage")
        != Some(&Value::Bool(false))
    {
        return Err(format!("{label} is incomplete"));
    }
    Ok(())
}

pub(super) fn array<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a Vec<Value>, String> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} nodes must be an array"))
}

pub(super) fn relative_path(
    object: &Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<String, String> {
    let path = text(object, key, label)?;
    if path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(format!("{label} {key} must be repository-relative"));
    }
    Ok(path.to_owned())
}

pub(super) fn is_oid(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
