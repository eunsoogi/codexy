use serde_json::{Map, Value};

pub(super) fn required_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("review control state {context} must contain non-empty {key}"))
}

pub(super) fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(format!(
            "review control state {context} contains an unknown field"
        ));
    }
    Ok(())
}
