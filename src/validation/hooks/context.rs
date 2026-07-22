#[cfg(unix)]
pub(super) mod process;
#[cfg(windows)]
#[path = "context/process_windows.rs"]
pub(super) mod process;
#[cfg(unix)]
mod process_finish;
#[cfg(all(test, unix))]
mod process_tests;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use regex::Regex;
use serde_json::Value;

use crate::paths::display_relative;

use super::command;

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
    "codexy-issue-title-check.sh",
    "codexy-pr-title-check.sh",
    "codexy-pr-label-check.sh",
    "codexy-merge-message-check.sh",
    "--check-issue-title",
    "--check-pr-title",
    "--check-pr-labels",
    "--check-merge-message",
    "--expected-pr",
    "target base",
    "hook entrypoints",
    "available fallback",
    "separate dogfood defect",
];

const REQUIRED_READINESS_CONTEXT: &[&str] = &[
    "PR label readiness enforcement (#210)",
    "codexy-issue-title-check.sh",
    "--check-issue-title",
    "--check-pr-labels",
    "codexy-pr-title-check.sh",
    "codexy-pr-label-check.sh",
    "codexy-merge-message-check.sh",
    "--check-completion-handoff",
    "repositoryLabels",
    "PR title and merge subject enforcement (#206)",
    "target base",
    "hook entrypoints",
    "available fallback",
    "separate dogfood defect",
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
        "--check-pr-labels" => "must require PR label readiness guard",
        "PR label readiness enforcement (#210)" => "must require PR label readiness enforcement",
        "target base" | "hook entrypoints" => "must require target-base hook entrypoint validation",
        "available fallback" | "separate dogfood defect" => {
            "must require hook fallback or mismatch defect routing"
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
        &[required_event],
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

pub(super) fn check_session_start_context(
    path: &Path,
    plugin_root: &Path,
    command_text: &str,
    timeout_secs: u64,
    required_event: &str,
    session_start_script: &str,
) -> Result<()> {
    let (hook_path, arguments) = command::plugin_root_entrypoint_path(command_text).with_context(
        || {
            format!(
                "{} {required_event} hook command must start with a packaged ${{PLUGIN_ROOT}} entrypoint",
                display_relative(path)
            )
        },
    )?;
    if hook_path != Path::new(session_start_script) {
        bail!(
            "{} {required_event} hook command must run {session_start_script}",
            display_relative(path)
        );
    }
    if !arguments
        .split_ascii_whitespace()
        .eq(std::iter::once(required_event))
    {
        bail!(
            "{} {required_event} hook command must invoke {required_event} exactly",
            display_relative(path)
        );
    }
    let script_path = plugin_root.join(&hook_path);
    let context = emitted_session_start_context(&script_path, required_event, timeout_secs)?;
    for fragment in required_session_start_context() {
        if !context.contains(fragment) {
            bail!(
                "{} {required_event} emitted additionalContext {}: {}",
                display_relative(path),
                requirement_message(fragment),
                display_relative(&script_path)
            );
        }
    }
    Ok(())
}

pub(super) fn check_readiness_context(
    path: &Path,
    plugin_root: &Path,
    command_text: &str,
    timeout_secs: u64,
    readiness_event: &str,
) -> Result<()> {
    let (hook_path, arguments) = command::plugin_root_entrypoint_path(command_text).with_context(|| {
        format!(
            "{} {readiness_event} hook command must start with a packaged ${{PLUGIN_ROOT}} entrypoint",
            display_relative(path)
        )
    })?;
    if !arguments
        .split_ascii_whitespace()
        .eq(std::iter::once(readiness_event))
    {
        bail!(
            "{} {readiness_event} hook command must invoke {readiness_event} exactly",
            display_relative(path)
        );
    }
    let script_path = plugin_root.join(&hook_path);
    let context = emitted_session_start_context(&script_path, readiness_event, timeout_secs)?;
    for fragment in REQUIRED_READINESS_CONTEXT {
        if !context.contains(fragment) {
            bail!(
                "{} {readiness_event} emitted additionalContext {}: {}",
                display_relative(path),
                requirement_message(fragment),
                display_relative(&script_path)
            );
        }
    }
    Ok(())
}

pub(super) fn session_start_covers_resume_and_compact(
    path: &Path,
    matcher: Option<&str>,
) -> Result<bool> {
    let Some(matcher) = matcher else {
        return Ok(true);
    };
    if matcher.is_empty() || matcher == "*" {
        return Ok(true);
    }
    let regex = Regex::new(matcher).with_context(|| {
        format!(
            "{} SessionStart.matcher must be a valid regex",
            display_relative(path)
        )
    })?;
    Ok(regex.is_match("resume") && regex.is_match("compact"))
}
