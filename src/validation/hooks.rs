mod command;

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::load_json;

const HOOKS_PATH: &str = "hooks/hooks.json";
const REQUIRED_EVENT: &str = "SessionStart";
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
    if let Some(matcher) = object.get("matcher")
        && !matcher
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
    {
        bail!(
            "{} {event}.matcher must be a non-empty string when present",
            display_relative(path)
        );
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
    if let Some(timeout) = object.get("timeout") {
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
    }
    if let Some(status) = object.get("statusMessage")
        && !status
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
    {
        bail!(
            "{} {event} hook statusMessage must be a non-empty string when present",
            display_relative(path)
        );
    }
    Ok(())
}
