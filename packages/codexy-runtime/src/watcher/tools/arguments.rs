use anyhow::{Context as _, Result, bail};
use serde_json::Value;

pub(super) fn identity(
    args: &serde_json::Map<String, Value>,
    primary: &str,
    legacy: &str,
) -> Result<Value> {
    if args.contains_key(primary) && args.contains_key(legacy) {
        bail!("watcher identities must use only one of {primary} or {legacy}");
    }
    args.get(primary)
        .or_else(|| args.get(legacy))
        .cloned()
        .context(format!("watcher {primary} identity is required"))
}

pub(super) fn ensure_keys(args: &serde_json::Map<String, Value>, allowed: &[&str]) -> Result<()> {
    if let Some(key) = args.keys().find(|key| !allowed.contains(&key.as_str())) {
        bail!("watcher argument is not supported: {key}");
    }
    Ok(())
}

pub(super) fn required_string(value: Option<&Value>, label: &str) -> Result<String> {
    let text = value
        .and_then(Value::as_str)
        .context(format!("watcher {label} must be a string"))?;
    if text.is_empty() || text.len() > 128 || text.chars().any(char::is_control) {
        bail!("watcher {label} is invalid");
    }
    Ok(text.to_owned())
}

pub(super) fn optional_u64(value: Option<&Value>, label: &str) -> Result<Option<u64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let number = value
        .as_u64()
        .context(format!("watcher {label} must be an integer"))?;
    Ok(Some(number))
}

pub(super) fn optional_cursor(value: Option<&Value>) -> Result<Option<u64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value {
        Value::Number(_) => optional_u64(Some(value), "cursor"),
        Value::String(text)
            if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            text.parse::<u64>()
                .map(Some)
                .context("watcher cursor is out of range")
        }
        _ => bail!("watcher cursor must be a non-negative integer or decimal string"),
    }
}

pub(super) fn event_field<'a>(
    args: &'a serde_json::Map<String, Value>,
    event: Option<&'a serde_json::Map<String, Value>>,
    key: &str,
) -> Option<&'a Value> {
    event
        .and_then(|object| object.get(key))
        .or_else(|| args.get(key))
}

pub(super) fn optional_event_string(
    args: &serde_json::Map<String, Value>,
    event: Option<&serde_json::Map<String, Value>>,
    key: &str,
) -> Result<Option<String>> {
    event_field(args, event, key)
        .map(|value| required_string(Some(value), key))
        .transpose()
}

pub(super) fn optional_event_u64(
    args: &serde_json::Map<String, Value>,
    event: Option<&serde_json::Map<String, Value>>,
    key: &str,
) -> Result<Option<u64>> {
    optional_u64(event_field(args, event, key), key)
}
