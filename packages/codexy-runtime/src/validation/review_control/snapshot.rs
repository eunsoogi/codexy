use std::collections::HashSet;

use serde_json::{Map, Value};

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
    reject_unknown(capture, &["provider", "method", "authenticated"], "capture")?;
    if required_text(capture, "provider", "capture")? != "github"
        || required_text(capture, "method", "capture")? != "graphql"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err(format!(
            "review control {label} PR snapshot capture is not authenticated GitHub GraphQL"
        ));
    }
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
