use std::collections::HashSet;

use serde_json::{Map, Value};

pub(super) fn section<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("final disposition {label} must be an object"))?;
    let allowed = match label {
        "source repair" => ["from_head", "evidence_commit", "finding_ids"].as_slice(),
        "evidence refresh" => ["proof_head", "finding_ids", "public_summaries"].as_slice(),
        _ => &[] as &[&str],
    };
    reject_unknown(object, allowed, label)?;
    Ok(object)
}

pub(super) fn section_ids(object: &Map<String, Value>, label: &str) -> Result<Vec<String>, String> {
    let ids = object
        .get("finding_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("final disposition {label} must list finding_ids"))?;
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.as_str().filter(|id| !id.is_empty()).ok_or_else(|| {
            format!("final disposition {label} finding_ids must be non-empty strings")
        })?;
        if !seen.insert(id) {
            return Err(format!(
                "final disposition {label} finding_ids must be unique"
            ));
        }
        result.push(id.to_owned());
    }
    if result.is_empty() {
        return Err(format!("final disposition {label} must cover a finding"));
    }
    Ok(result)
}

pub(super) fn finding_ids(findings: &[Value], label: &str) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();
    for finding in findings {
        let finding = finding
            .as_object()
            .ok_or_else(|| format!("{label} must contain finding objects"))?;
        let id = text(finding, "id", label)?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{label} must contain unique finding ids"));
        }
    }
    Ok(ids)
}

pub(super) fn oid<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    let value = text(object, key, label)?;
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "final disposition {label} {key} must be a commit SHA"
        ));
    }
    Ok(value)
}

pub(super) fn text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("final disposition {label} must contain non-empty {key}"))
}

pub(super) fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> Result<(), String> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(format!(
            "final disposition {label} contains an unknown field"
        ));
    }
    Ok(())
}
