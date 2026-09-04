use std::{path::Path, process::Command};

use serde_json::{Map, Value};

pub(super) fn check(
    repository_root: &Path,
    from: &str,
    to: &str,
    findings: &[Value],
) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repository_root)
        .args(["diff", "--name-only", "--no-ext-diff", from, to, "--"])
        .output()
        .map_err(|error| {
            format!("review control cannot inspect qualifying repair diff: {error}")
        })?;
    if !output.status.success() {
        return Err("review control cannot inspect qualifying repair diff".into());
    }
    let changed = String::from_utf8_lossy(&output.stdout);
    if changed.lines().next().is_none() {
        return Err("contract/root repair evidence must change the reviewed tree".into());
    }
    for finding in findings {
        let object = finding
            .as_object()
            .ok_or_else(|| "prior delta findings must contain finding objects".to_owned())?;
        let path = required_text(object, "path", "prior delta finding")?;
        if !changed.lines().any(|changed| changed == path) {
            return Err(format!(
                "qualifying change evidence is not linked to prior finding path: {path}"
            ));
        }
    }
    Ok(())
}

fn required_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("review control {label} must contain non-empty {key}"))
}
