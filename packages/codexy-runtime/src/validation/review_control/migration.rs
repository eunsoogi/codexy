use serde_json::{Map, Value, json};

use super::policy;

pub(super) fn boundary(
    control: &Map<String, Value>,
    profile: &str,
    current_reviewer: &Value,
    history_len: usize,
) -> Result<Option<usize>, String> {
    let Some(value) = control.get("reviewer_migration") else {
        return Ok(None);
    };
    let object = value
        .as_object()
        .ok_or_else(|| "review control state reviewer_migration must be an object".to_owned())?;
    if object
        .keys()
        .any(|key| !["schema", "from", "to", "history_boundary"].contains(&key.as_str()))
    {
        return Err("review control state reviewer_migration contains an unknown field".into());
    }
    if object.get("schema").and_then(Value::as_str) != Some(policy::REVIEWER_MIGRATION_SCHEMA) {
        return Err("review control state reviewer_migration has an unsupported schema".into());
    }
    let legacy_reviewer = policy::legacy_reviewer(profile)
        .ok_or_else(|| "reviewer migration is not supported for this profile".to_owned())?;
    if object.get("from") != Some(&legacy_reviewer) {
        return Err("review control state reviewer_migration has an invalid predecessor".into());
    }
    if object.get("to") != Some(current_reviewer) {
        return Err("review control state reviewer_migration has an invalid successor".into());
    }
    let boundary = object
        .get("history_boundary")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            "review control state reviewer_migration must contain numeric history_boundary"
                .to_owned()
        })?;
    let boundary = usize::try_from(boundary)
        .map_err(|_| "review control state reviewer_migration boundary is too large".to_owned())?;
    if boundary == 0 || boundary >= history_len {
        return Err("review control state reviewer_migration boundary is invalid".into());
    }
    Ok(Some(boundary))
}

pub(super) fn marker(
    profile: &str,
    current_reviewer: &Value,
    history_boundary: u64,
) -> Result<Value, String> {
    let legacy_reviewer = policy::legacy_reviewer(profile)
        .ok_or_else(|| "reviewer migration is not supported for this profile".to_owned())?;
    Ok(json!({
        "schema": policy::REVIEWER_MIGRATION_SCHEMA,
        "from": legacy_reviewer,
        "to": current_reviewer,
        "history_boundary": history_boundary
    }))
}

pub(super) fn reconcile(
    current: &mut Map<String, Value>,
    expected: Option<Value>,
) -> Result<(), String> {
    match (current.get("reviewer_migration"), expected) {
        (Some(actual), Some(expected)) if actual == &expected => Ok(()),
        (Some(_), Some(_)) => Err("review control transition changes reviewer migration".into()),
        (Some(_), None) => Err("review control transition adds reviewer migration".into()),
        (None, Some(expected)) => {
            current.insert("reviewer_migration".into(), expected);
            Ok(())
        }
        (None, None) => Ok(()),
    }
}
