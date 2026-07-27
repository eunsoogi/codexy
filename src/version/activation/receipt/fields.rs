use anyhow::{Context as _, Result, bail};
use serde_json::{Map, Value};

pub(super) fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .with_context(|| format!("{label} must be an object"))
}

pub(super) fn object_field<'a>(
    value: &'a Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<&'a Map<String, Value>> {
    value
        .get(field)
        .and_then(Value::as_object)
        .with_context(|| format!("{label} {field} must be an object"))
}

pub(super) fn string<'a>(
    value: &'a Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{label} {field} must be a non-empty string"))
}

pub(super) fn exact_keys(value: &Map<String, Value>, expected: &[&str], label: &str) -> Result<()> {
    let mut keys = value.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    if keys == expected {
        Ok(())
    } else {
        bail!("{label} has unknown or missing fields")
    }
}

pub(super) fn exact(actual: &str, expected: &str, label: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        bail!("{label} is not canonical")
    }
}

pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        bail!("digest must be a lowercase SHA-256")
    }
}

pub(super) fn commit(value: &str) -> Result<()> {
    if value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        bail!("commit must be lowercase 40-character SHA")
    }
}

pub(super) fn tag(value: &str) -> Result<()> {
    let slug = value.strip_prefix("runtime-candidate-").unwrap_or_default();
    if !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        Ok(())
    } else {
        bail!("candidate tag must have a safe runtime-candidate slug")
    }
}

pub(super) fn binary_path(value: &str, server: &str, platform: &str) -> Result<()> {
    let expected = format!("runtime/codexy-mcp-{server}-{platform}.bin");
    if value == expected {
        Ok(())
    } else {
        bail!("candidate binary path is not canonical")
    }
}

pub(super) fn positive_integer(
    value: &Map<String, Value>,
    field: &str,
    label: &str,
) -> Result<i64> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .with_context(|| format!("{label} {field} must be a positive integer"))
}
