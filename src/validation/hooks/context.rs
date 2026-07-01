mod process;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;

const REQUIRED_SESSION_START_CONTEXT: &[&str] = &[
    "codegraph MCP before direct file reads",
    "include codegraph findings",
    "codegraph unavailable/uncallable fallback evidence",
    "registered-but-uncallable/unavailable-tool evidence",
    "Use Codexy LSP",
    "lsp_status",
    "unavailable/not applicable evidence",
    "$dreaming",
    "compacted or resumed context hygiene",
    "--check-completion-handoff",
    "repositoryLabels",
];

pub(super) fn required_session_start_context() -> &'static [&'static str] {
    REQUIRED_SESSION_START_CONTEXT
}

pub(super) fn requirement_message(fragment: &str) -> &str {
    match fragment {
        "codegraph MCP before direct file reads" | "include codegraph findings" => {
            "must require codegraph evidence"
        }
        "codegraph unavailable/uncallable fallback evidence"
        | "registered-but-uncallable/unavailable-tool evidence" => {
            "must require codegraph fallback evidence"
        }
        "Use Codexy LSP" | "lsp_status" => "must require LSP evidence",
        "unavailable/not applicable evidence" => "must require LSP fallback evidence",
        "$dreaming" | "compacted or resumed context hygiene" => "must require dreaming hygiene",
        "--check-completion-handoff" | "repositoryLabels" => {
            "must require PR label readiness validation"
        }
        _ => "must include required context",
    }
}

pub(super) fn emitted_session_start_context(
    script_path: &Path,
    required_event: &str,
    timeout_secs: u64,
) -> Result<String> {
    let output = process::output_with_timeout(
        script_path,
        required_event,
        Duration::from_secs(timeout_secs),
    )?;
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
