use serde_json::{Map, Value};

use super::super::source::fields;

pub(super) fn validate(
    snapshot: &Map<String, Value>,
    target: &Map<String, Value>,
) -> Result<(), String> {
    let repository = required_text(snapshot, "repository")?;
    let target_repository = required_text(target, "repository")?;
    if repository != target_repository {
        return Err("current PR snapshot repository does not match target".into());
    }
    let number = required_u64(snapshot, "number")?;
    if Some(number) != target.get("pullRequest").and_then(Value::as_u64) {
        return Err("current PR snapshot pull request does not match target".into());
    }
    if let Some(issue) = optional_u64(snapshot, &["owningIssue", "issueNumber", "issue_number"])?
        && Some(issue) != target.get("owningIssue").and_then(Value::as_u64)
    {
        return Err("current PR snapshot owning issue does not match target".into());
    }
    let url = required_text(snapshot, "url")?;
    if url != format!("https://github.com/{repository}/pull/{number}") {
        return Err("current PR snapshot URL does not bind its identity".into());
    }
    for key in ["baseRefName", "baseRefOid", "headRefOid"] {
        required_text(snapshot, key)?;
    }
    for key in ["baseRefOid", "headRefOid"] {
        if !fields::is_sha(required_text(snapshot, key)?) {
            return Err(format!("current PR snapshot {key} must be a commit SHA"));
        }
    }
    let capture = snapshot
        .get("capture")
        .and_then(Value::as_object)
        .ok_or("current PR snapshot must contain capture")?;
    if required_text(capture, "provider")? != "github"
        || required_text(capture, "method")? != "graphql"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err("current PR snapshot capture is not authenticated GitHub GraphQL".into());
    }
    Ok(())
}

pub(super) fn temporal(
    receipt: &Map<String, Value>,
    snapshot: &Map<String, Value>,
) -> Result<String, String> {
    let created = snapshot
        .get("createdAtEpoch")
        .or_else(|| snapshot.get("createdAt"))
        .and_then(Value::as_u64);
    let events = receipt
        .get("events")
        .and_then(Value::as_array)
        .ok_or("native-history receipt must contain events")?;
    let Some(created) = created else {
        return Ok("unresolved".into());
    };
    let mut all_observed = true;
    for event in events {
        let completed = event.get("completed_at").and_then(Value::as_u64);
        let Some(completed) = completed else {
            all_observed = false;
            continue;
        };
        if completed <= created {
            return Err("native-history event completed before or at PR creation".into());
        }
    }
    Ok(if all_observed {
        "proved_post_pr"
    } else {
        "unresolved"
    }
    .into())
}

fn required_text<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    match map.get(key) {
        Some(Value::String(value)) if !value.is_empty() => Ok(value),
        Some(Value::Null) | None => Err(format!("snapshot must contain {key}")),
        Some(_) => Err(format!("{key} must be a non-empty string")),
    }
}

fn required_u64(map: &Map<String, Value>, key: &str) -> Result<u64, String> {
    map.get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("snapshot must contain positive numeric {key}"))
}

fn optional_u64(map: &Map<String, Value>, keys: &[&str]) -> Result<Option<u64>, String> {
    let mut found = None;
    for key in keys {
        if let Some(value) = map.get(*key) {
            let Some(number) = value.as_u64() else {
                if value.is_null() {
                    continue;
                }
                return Err(format!("{key} must be numeric"));
            };
            if found.is_some_and(|prior| prior != number) {
                return Err("snapshot issue aliases are contradictory".into());
            }
            found = Some(number);
        }
    }
    Ok(found)
}
