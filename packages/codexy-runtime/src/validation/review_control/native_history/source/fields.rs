use serde_json::{Map, Value};

pub(super) fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

pub(super) fn required(value: Option<String>, label: &str) -> Result<String, String> {
    value
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label} must be non-empty"))
}

pub(super) fn text(
    map: &Map<String, Value>,
    aliases: &[&str],
    label: &str,
) -> Result<Option<String>, String> {
    let value = value(map, aliases, label)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value)),
        Some(_) => Err(format!("{label} must be a non-empty string")),
    }
}

pub(super) fn text_array(
    map: &Map<String, Value>,
    aliases: &[&str],
    label: &str,
) -> Result<Option<Vec<String>>, String> {
    let Some(value) = value(map, aliases, label)? else {
        return Ok(None);
    };
    let array = value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))?;
    array
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("{label} must contain non-empty strings"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

pub(super) fn value(
    map: &Map<String, Value>,
    aliases: &[&str],
    label: &str,
) -> Result<Option<Value>, String> {
    let mut found = None;
    for alias in aliases {
        if let Some(value) = map.get(*alias) {
            if found.as_ref().is_some_and(|prior| prior != value) {
                return Err(format!("{label} has contradictory aliases"));
            }
            found = Some(value.clone());
        }
    }
    Ok(found)
}

pub(super) fn merged_value(
    objects: &[Map<String, Value>],
    aliases: &[&str],
    label: &str,
) -> Result<Option<Value>, String> {
    let mut found = None;
    for object in objects {
        if let Some(value) = value(object, aliases, label)? {
            if found.as_ref().is_some_and(|prior| prior != &value) {
                return Err(format!("{label} is contradictory"));
            }
            found = Some(value);
        }
    }
    Ok(found)
}

pub(super) fn merged_text(
    objects: &[Map<String, Value>],
    aliases: &[&str],
    label: &str,
) -> Result<Option<String>, String> {
    let mut found = None;
    for object in objects {
        if let Some(value) = text(object, aliases, label)? {
            if found.as_ref().is_some_and(|prior| prior != &value) {
                return Err(format!("{label} is contradictory"));
            }
            found = Some(value);
        }
    }
    Ok(found)
}

pub(super) fn merged_u64(
    objects: &[Map<String, Value>],
    aliases: &[&str],
    label: &str,
) -> Result<Option<u64>, String> {
    let Some(value) = merged_value(objects, aliases, label)? else {
        return Ok(None);
    };
    match value {
        Value::Null => Ok(None),
        Value::Number(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{label} must be a non-negative integer")),
        _ => Err(format!("{label} must be numeric")),
    }
}

pub(super) fn collect(value: &Value, index: usize, result: &mut Vec<(usize, Value)>) {
    if value.is_object() {
        result.push((index, value.clone()));
    }
    if let Some(array) = value.as_array() {
        for child in array {
            collect(child, index, result);
        }
    }
    if let Some(map) = value.as_object() {
        for child in map.values() {
            collect(child, index, result);
        }
    }
}

pub(crate) fn is_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn derived_id(message_id: &str, span: &str, value: &Value) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{message_id}|{span}|{value}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("derived-{hash:016x}")
}
