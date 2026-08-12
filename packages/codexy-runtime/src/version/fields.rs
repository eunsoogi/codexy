use anyhow::{Context as _, Result};
use serde_json::Value;

pub(super) fn string_array(data: &Value, field: &str, label: &str) -> Result<Vec<String>> {
    let values = data
        .get(field)
        .and_then(Value::as_array)
        .with_context(|| format!("{label} {field} must be an array"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|item| !item.trim().is_empty())
                .map(ToOwned::to_owned)
                .with_context(|| format!("{label} {field} must contain only non-empty strings"))
        })
        .collect()
}
