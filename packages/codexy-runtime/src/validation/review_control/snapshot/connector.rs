use serde_json::{Map, Value};

use super::{reject_unknown, required_oid, required_text};

const TOOL: &str = "mcp__codex_apps__github_get_pr_info";

pub(super) fn check(
    capture: &Map<String, Value>,
    snapshot: &Map<String, Value>,
    label: &str,
) -> Result<(), String> {
    reject_unknown(
        capture,
        &[
            "provider",
            "method",
            "authenticated",
            "source",
            "owningIssue",
        ],
        "connector capture",
    )?;
    let projection = project(capture, snapshot, label)?;
    for (field, value) in projection {
        if snapshot.get(&field) != Some(&value) {
            return Err(format!(
                "review control {label} PR snapshot {field} disagrees with connector source"
            ));
        }
    }
    Ok(())
}

pub(super) fn normalize(value: &Value, label: &str) -> Result<Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("review control {label} PR snapshot must be an object"))?;
    let capture = object
        .get("capture")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!("review control {label} PR snapshot must carry capture provenance")
        })?;
    check_capture(capture, label)?;
    let projection = project(capture, object, label)?;
    let mut normalized = value.clone();
    let normalized_object = normalized
        .as_object_mut()
        .ok_or_else(|| format!("review control {label} PR snapshot must be an object"))?;
    for (field, value) in projection {
        if let Some(existing) = normalized_object.get(&field) {
            if existing != &value {
                return Err(format!(
                    "review control {label} PR snapshot {field} disagrees with connector source"
                ));
            }
        } else {
            normalized_object.insert(field, value);
        }
    }
    Ok(normalized)
}

fn check_capture(capture: &Map<String, Value>, label: &str) -> Result<(), String> {
    reject_unknown(
        capture,
        &[
            "provider",
            "method",
            "authenticated",
            "source",
            "owningIssue",
        ],
        "connector capture",
    )?;
    if required_text(capture, "provider", "capture")? != "github"
        || required_text(capture, "method", "capture")? != "connector"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err(format!(
            "review control {label} PR snapshot capture is not authenticated GitHub connector"
        ));
    }
    Ok(())
}

fn project(
    capture: &Map<String, Value>,
    snapshot: &Map<String, Value>,
    label: &str,
) -> Result<Map<String, Value>, String> {
    let source = capture
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "review control {label} PR snapshot connector capture must carry source provenance"
            )
        })?;
    reject_unknown(source, &["tool", "arguments", "result"], "connector source")?;
    if required_text(source, "tool", "connector source")? != TOOL {
        return Err(format!(
            "review control {label} PR snapshot connector tool is unsupported"
        ));
    }
    let arguments = source
        .get("arguments")
        .and_then(Value::as_object)
        .ok_or_else(|| "review control connector source arguments must be an object".to_owned())?;
    reject_unknown(
        arguments,
        &["repository_full_name", "pr_number"],
        "connector source arguments",
    )?;
    let repository = required_text(arguments, "repository_full_name", "connector arguments")?;
    if let Some(snapshot_repository) = snapshot.get("repository") {
        if snapshot_repository.as_str() != Some(repository) {
            return Err(format!(
                "review control {label} PR snapshot connector arguments change repository identity"
            ));
        }
    }
    let number = arguments
        .get("pr_number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| {
            "review control connector arguments must contain positive pr_number".to_owned()
        })?;
    if let Some(snapshot_number) = snapshot.get("number") {
        if snapshot_number != &Value::from(number) {
            return Err(format!(
                "review control {label} PR snapshot connector arguments change PR identity"
            ));
        }
    }
    let result = source
        .get("result")
        .and_then(Value::as_object)
        .ok_or_else(|| "review control connector source result must be an object".to_owned())?;
    let result_number = result
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| "review control connector result must contain number".to_owned())?;
    if result_number != number {
        return Err(format!(
            "review control {label} PR snapshot connector result changes PR identity"
        ));
    }
    if let Some(result_repository) = result.get("repository") {
        if result_repository.as_str() != Some(repository) {
            return Err(format!(
                "review control {label} PR snapshot connector result changes repository identity"
            ));
        }
    }
    let url = required_text(result, "url", "connector result")?;
    if url != format!("https://github.com/{repository}/pull/{number}") {
        return Err(format!(
            "review control {label} PR snapshot connector result URL does not bind the PR identity"
        ));
    }
    let base_ref_name = required_text(result, "base", "connector result")?;
    let base_ref_oid = oid(result, "base_sha", "connector result")?;
    let head_ref_oid = oid(result, "head_sha", "connector result")?;
    Ok(Map::from_iter([
        ("repository".into(), Value::String(repository.to_owned())),
        ("number".into(), Value::from(number)),
        ("url".into(), Value::String(url.to_owned())),
        (
            "baseRefName".into(),
            Value::String(base_ref_name.to_owned()),
        ),
        ("baseRefOid".into(), Value::String(base_ref_oid.to_owned())),
        ("headRefOid".into(), Value::String(head_ref_oid.to_owned())),
    ]))
}

fn oid<'a>(object: &'a Map<String, Value>, key: &str, label: &str) -> Result<&'a str, String> {
    let value = required_oid(object, key, label)?;
    Ok(value)
}
