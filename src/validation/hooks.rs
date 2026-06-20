mod command;

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::load_json;

const HOOKS_PATH: &str = "hooks/hooks.json";
const REQUIRED_EVENT: &str = "SessionStart";
const SESSION_START_SCRIPT: &str = "hooks/codexy-routing-context.sh";
const REQUIRED_SESSION_START_CONTEXT: &[(&str, &str)] = &[
    (
        "codegraph MCP before direct file reads",
        "must require codegraph evidence",
    ),
    (
        "include codegraph findings",
        "must require codegraph evidence",
    ),
    (
        "registered-but-uncallable/unavailable-tool evidence",
        "must require codegraph fallback evidence",
    ),
    ("Use Codexy LSP", "must require LSP evidence"),
    ("lsp_status", "must require LSP evidence"),
    (
        "unavailable/not applicable evidence",
        "must require LSP fallback evidence",
    ),
];
const ALLOWED_EVENTS: &[&str] = &[
    "PermissionRequest",
    "PostCompact",
    "PostToolUse",
    "PreCompact",
    "PreToolUse",
    "SessionStart",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "UserPromptSubmit",
];
pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    match check_inner(plugin_root) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

fn check_inner(plugin_root: &Path) -> Result<()> {
    let path = plugin_root.join(HOOKS_PATH);
    let data = load_json(&path)?;
    let events = data
        .get("hooks")
        .and_then(Value::as_object)
        .with_context(|| format!("{} hooks must be an object", display_relative(&path)))?;
    if !events.contains_key(REQUIRED_EVENT) {
        bail!(
            "{} must define a {REQUIRED_EVENT} hook",
            display_relative(&path)
        );
    }
    for (event, groups) in events {
        if !ALLOWED_EVENTS.contains(&event.as_str()) {
            bail!(
                "{} unsupported hook event: {event}",
                display_relative(&path)
            );
        }
        let groups = groups
            .as_array()
            .filter(|items| !items.is_empty())
            .with_context(|| {
                format!(
                    "{} {event} must be a non-empty matcher group array",
                    display_relative(&path)
                )
            })?;
        for group in groups {
            check_group(&path, plugin_root, event, group)?;
        }
    }
    Ok(())
}

fn check_group(path: &Path, plugin_root: &Path, event: &str, group: &Value) -> Result<()> {
    let object = group
        .as_object()
        .with_context(|| format!("{} {event} group must be an object", display_relative(path)))?;
    if let Some(matcher) = object.get("matcher") {
        if !matcher
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
        {
            bail!(
                "{} {event}.matcher must be a non-empty string when present",
                display_relative(path)
            );
        }
    }
    let handlers = object
        .get("hooks")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .with_context(|| {
            format!(
                "{} {event}.hooks must be a non-empty array",
                display_relative(path)
            )
        })?;
    for handler in handlers {
        check_handler(path, plugin_root, event, handler)?;
    }
    Ok(())
}

fn check_handler(path: &Path, plugin_root: &Path, event: &str, handler: &Value) -> Result<()> {
    let object = handler.as_object().with_context(|| {
        format!(
            "{} {event} hook handler must be an object",
            display_relative(path)
        )
    })?;
    if object.get("type").and_then(Value::as_str) != Some("command") {
        bail!(
            "{} {event} hook handlers must use type \"command\"",
            display_relative(path)
        );
    }
    if let Some(async_value) = object.get("async") {
        match async_value.as_bool() {
            Some(false) => {}
            Some(true) => bail!(
                "{} {event} hook handlers must not set async=true",
                display_relative(path)
            ),
            None => bail!(
                "{} {event} hook async must be a boolean when present",
                display_relative(path)
            ),
        }
    }
    let command = object
        .get("command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .with_context(|| {
            format!(
                "{} {event} hook command must be a non-empty string",
                display_relative(path)
            )
        })?;
    command::check_command(path, plugin_root, event, command)?;
    if event == REQUIRED_EVENT {
        check_session_start_context(path, plugin_root, command)?;
    }
    let timeout = object.get("timeout").with_context(|| {
        format!(
            "{} {event} hook timeout is required",
            display_relative(path)
        )
    })?;
    let timeout = timeout.as_u64().with_context(|| {
        format!(
            "{} {event} hook timeout must be a positive integer",
            display_relative(path)
        )
    })?;
    if timeout == 0 || timeout > 10 {
        bail!(
            "{} {event} hook timeout must be between 1 and 10 seconds",
            display_relative(path)
        );
    }
    if let Some(status) = object.get("statusMessage") {
        if !status
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
        {
            bail!(
                "{} {event} hook statusMessage must be a non-empty string when present",
                display_relative(path)
            );
        }
    }
    Ok(())
}

fn check_session_start_context(path: &Path, plugin_root: &Path, command: &str) -> Result<()> {
    let (hook_path, _) = command::plugin_root_entrypoint_path(command).with_context(|| {
        format!(
            "{} {REQUIRED_EVENT} hook command must start with a packaged ${{PLUGIN_ROOT}} entrypoint",
            display_relative(path)
        )
    })?;
    if hook_path != Path::new(SESSION_START_SCRIPT) {
        bail!(
            "{} {REQUIRED_EVENT} hook command must run {SESSION_START_SCRIPT}",
            display_relative(path)
        );
    }
    let script_path = plugin_root.join(&hook_path);
    let script = std::fs::read_to_string(&script_path)
        .with_context(|| format!("reading {}", display_relative(&script_path)))?;
    for (fragment, message) in REQUIRED_SESSION_START_CONTEXT {
        if !script.contains(fragment) {
            bail!(
                "{} {REQUIRED_EVENT} routing context {message}: {}",
                display_relative(path),
                display_relative(&script_path)
            );
        }
    }
    Ok(())
}
