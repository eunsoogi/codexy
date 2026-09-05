use std::collections::HashSet;

use serde_json::{Map, Value};

const ISSUE_ASSOCIATIONS: [&str; 3] = [
    "owner-assignment",
    "closing-issue-reference",
    "linked-issue-reference",
];

pub(super) fn check(value: &Value, label: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("review control {label} PR snapshot must be an object"))?;
    let repository = required_text(object, "repository", label)?;
    if repository.split('/').count() != 2 || repository.split('/').any(str::is_empty) {
        return Err(format!(
            "review control {label} PR snapshot repository identity is invalid"
        ));
    }
    let number = object
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| format!("review control {label} PR snapshot must contain number"))?;
    required_text(object, "baseRefName", label)?;
    required_oid(object, "baseRefOid", label)?;
    required_oid(object, "headRefOid", label)?;
    let url = required_text(object, "url", label)?;
    if url != format!("https://github.com/{repository}/pull/{number}") {
        return Err(format!(
            "review control {label} PR snapshot URL does not bind the PR identity"
        ));
    }
    let capture = object
        .get("capture")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!("review control {label} PR snapshot must carry capture provenance")
        })?;
    reject_unknown(
        capture,
        &["provider", "method", "authenticated", "owningIssue"],
        "capture",
    )?;
    if required_text(capture, "provider", "capture")? != "github"
        || required_text(capture, "method", "capture")? != "graphql"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err(format!(
            "review control {label} PR snapshot capture is not authenticated GitHub GraphQL"
        ));
    }
    let owning_issue = capture
        .get("owningIssue")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!("review control {label} PR snapshot capture must carry owning issue identity")
        })?;
    check_owning_issue(owning_issue, repository, label)?;
    Ok(())
}

pub(super) fn same_pr(previous: &Value, current: &Value) -> Result<(), String> {
    for field in ["repository", "number", "url", "baseRefName"] {
        if previous.get(field) != current.get(field) {
            return Err(format!(
                "review control PR snapshots change authenticated {field} identity"
            ));
        }
    }
    Ok(())
}

pub(super) fn same_issue(previous: &Value, current: &Value) -> Result<(), String> {
    let previous_issue = owning_issue(previous, "previous")?;
    let current_issue = owning_issue(current, "current")?;
    for field in ["repository", "number", "url"] {
        if previous_issue.get(field) != current_issue.get(field) {
            return Err(format!(
                "review control snapshots change authenticated owning issue {field}"
            ));
        }
    }
    Ok(())
}

pub(super) fn owning_issue_number(snapshot: &Value, label: &str) -> Result<u64, String> {
    owning_issue(snapshot, label)?
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| {
            format!("review control {label} PR snapshot owning issue must contain number")
        })
}

pub(super) fn required_oid<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    let value = required_text(object, key, label)?;
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "review control {label} PR snapshot {key} must be a commit SHA"
        ));
    }
    Ok(value)
}

pub(super) fn required_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("review control {label} PR snapshot must contain {key}"))
}

fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> Result<(), String> {
    let allowed = allowed.iter().copied().collect::<HashSet<_>>();
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(format!(
            "review control PR snapshot {label} contains an unknown field"
        ));
    }
    Ok(())
}

fn owning_issue<'a>(snapshot: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    snapshot
        .get("capture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("owningIssue"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!("review control {label} PR snapshot capture must carry owning issue identity")
        })
}

fn check_owning_issue(
    issue: &Map<String, Value>,
    pr_repository: &str,
    label: &str,
) -> Result<(), String> {
    reject_unknown(
        issue,
        &["repository", "number", "url", "association"],
        "owning issue",
    )?;
    let repository = issue_text(issue, "repository", label)?;
    if repository != pr_repository
        || repository.split('/').count() != 2
        || repository.split('/').any(str::is_empty)
    {
        return Err(format!(
            "review control {label} owning issue repository identity is invalid"
        ));
    }
    let number = issue
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| {
            format!("review control {label} PR snapshot owning issue must contain number")
        })?;
    if issue_text(issue, "url", label)?
        != format!("https://github.com/{repository}/issues/{number}")
    {
        return Err(format!(
            "review control {label} owning issue URL does not bind the issue identity"
        ));
    }
    let association = issue_text(issue, "association", label)?;
    if !ISSUE_ASSOCIATIONS.contains(&association) {
        return Err(format!(
            "review control {label} owning issue association is not authenticated"
        ));
    }
    Ok(())
}

fn issue_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!("review control {label} PR snapshot owning issue must contain {key}")
        })
}
