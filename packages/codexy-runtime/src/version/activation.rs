mod receipt;
mod updates;

use serde_json::Value;

use updates::{apply_with, prepare};

/// Validates a candidate receipt and atomically stages its activation updates.
/// No publication, commit, branch, or pull request action is performed here.
///
/// # Errors
///
/// Returns an error when the candidate receipt or activation targets are invalid,
/// or when atomic staging cannot complete.
pub fn activate(
    repo_root: &std::path::Path,
    bootstrap_version: &str,
    receipt_path: &std::path::Path,
) -> anyhow::Result<usize> {
    let updates = prepare(repo_root, bootstrap_version, receipt_path)?;
    apply_with(&updates, updates::write_staged)?;
    Ok(updates.len())
}

pub(super) fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonical(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}

#[cfg(test)]
#[path = "activation/tests.rs"]
mod tests;
