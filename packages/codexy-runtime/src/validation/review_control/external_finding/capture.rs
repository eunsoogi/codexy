use serde_json::{Map, Value};

use super::super::pre_pr::{object, reject_unknown, text};

const RAW_FIELDS: [&str; 8] = [
    "repository",
    "owningIssue",
    "pullRequest",
    "reviewThread",
    "reviewComment",
    "author",
    "observedCommit",
    "findings",
];

pub(super) fn check(
    capture: &Map<String, Value>,
    source: &Map<String, Value>,
) -> Result<(), String> {
    check_shape(capture, "external finding capture")?;
    let raw = object(capture.get("raw"), "external finding raw capture")?;
    for field in RAW_FIELDS {
        if source.get(field) != raw.get(field) {
            return Err("external finding source does not match raw authenticated capture".into());
        }
    }
    Ok(())
}

pub(super) fn bind(source: &Value, host_capture: &Value) -> Result<Value, String> {
    let source_object = object(Some(source), "authenticated external finding")?;
    let source_capture = object(source_object.get("capture"), "external finding capture")?;
    let host_capture = object(
        Some(host_capture),
        "authenticated external finding host capture",
    )?;
    reject_unknown(
        source_capture,
        &["provider", "method", "authenticated", "raw"],
        "external finding capture",
    )?;
    for field in ["provider", "method", "authenticated"] {
        if source_capture.get(field) != host_capture.get(field) {
            return Err("external finding capture metadata does not match host readback".into());
        }
    }
    check_shape(host_capture, "authenticated external finding host capture")?;
    let mut bound = source.clone();
    bound
        .as_object_mut()
        .ok_or_else(|| "authenticated external finding must be an object".to_owned())?
        .insert("capture".into(), Value::Object(host_capture.clone()));
    Ok(bound)
}

fn check_shape(capture: &Map<String, Value>, label: &str) -> Result<(), String> {
    reject_unknown(
        capture,
        &["provider", "method", "authenticated", "raw"],
        label,
    )?;
    if text(capture, "provider", label)? != "github"
        || text(capture, "method", label)? != "graphql"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err("external finding source is not authenticated GitHub GraphQL".into());
    }
    let raw_value = capture
        .get("raw")
        .ok_or_else(|| "external finding capture requires raw authenticated capture".to_owned())?;
    let raw = object(Some(raw_value), "external finding raw capture")?;
    reject_unknown(raw, &RAW_FIELDS, "external finding raw capture")?;
    Ok(())
}
