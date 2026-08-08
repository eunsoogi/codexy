use std::{collections::BTreeSet, path::Path};

use anyhow::{Context as _, Result, bail};
use serde_json::{Map, Value};

use crate::paths::display_relative;

pub(super) fn object<'a>(
    value: &'a Value,
    name: &str,
    path: &Path,
) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .with_context(|| format!("{} {name} must be an object", display_relative(path)))
}

pub(super) fn object_field<'a>(
    value: &'a Map<String, Value>,
    field: &str,
    path: &Path,
) -> Result<&'a Map<String, Value>> {
    object(
        value
            .get(field)
            .with_context(|| format!("{} missing {field}", display_relative(path)))?,
        field,
        path,
    )
}

pub(super) fn string<'a>(
    value: &'a Map<String, Value>,
    field: &str,
    path: &Path,
) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|item| !item.is_empty())
        .with_context(|| {
            format!(
                "{} {field} must be a non-empty string",
                display_relative(path)
            )
        })
}

pub(super) fn integer(
    value: &Map<String, Value>,
    field: &str,
    path: &Path,
    expected: i64,
) -> Result<()> {
    if value.get(field).and_then(Value::as_i64) == Some(expected) {
        Ok(())
    } else {
        bail!("{} {field} must be {expected}", display_relative(path))
    }
}

pub(super) fn digest<'a>(value: &'a str, field: &str, path: &Path) -> Result<&'a str> {
    if value.len() == 64
        && value.bytes().all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
    {
        Ok(value)
    } else {
        bail!(
            "{} {field} must be a lowercase SHA-256 digest",
            display_relative(path)
        )
    }
}

pub(super) fn exact(actual: &str, expected: &str, field: &str, path: &Path) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        bail!(
            "{} {field} must be {expected:?}, got {actual:?}",
            display_relative(path)
        )
    }
}

pub(super) fn exact_keys(value: &Map<String, Value>, expected: &[&str], path: &Path) -> Result<()> {
    let actual = value.keys().cloned().collect::<BTreeSet<_>>();
    let expected = expected
        .iter()
        .map(|item| (*item).to_owned())
        .collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        bail!(
            "{} has unknown or missing fields: expected {:?}, got {:?}",
            display_relative(path),
            expected,
            actual
        )
    }
}
