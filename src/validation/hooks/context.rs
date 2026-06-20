use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;

pub(super) fn emitted_session_start_context(
    script_path: &Path,
    required_event: &str,
) -> Result<String> {
    let output = Command::new(script_path)
        .arg(required_event)
        .output()
        .with_context(|| format!("running {}", display_relative(script_path)))?;
    if !output.status.success() {
        bail!(
            "{} {required_event} hook command failed: {}",
            display_relative(script_path),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let data: Value = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "{} {required_event} hook output must be JSON",
            display_relative(script_path)
        )
    })?;
    let event = data
        .get("hookSpecificOutput")
        .and_then(|value| value.get("hookEventName"))
        .and_then(Value::as_str);
    if event != Some(required_event) {
        bail!(
            "{} hook output must set hookSpecificOutput.hookEventName to {required_event}",
            display_relative(script_path)
        );
    }

    data.get("hookSpecificOutput")
        .and_then(|value| value.get("additionalContext"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} hook output must emit non-empty hookSpecificOutput.additionalContext",
                display_relative(script_path)
            )
        })
}
