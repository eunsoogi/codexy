mod command;
mod context;
mod lifecycle;
mod lifecycle_probe;

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::load_json;

const HOOKS_PATH: &str = "hooks/hooks.json";
const REQUIRED_EVENT: &str = "SessionStart";
const READINESS_EVENT: &str = "UserPromptSubmit";
const SESSION_START_SCRIPT: &str = "hooks/codexy-routing-context.sh";
const READINESS_SCRIPT: &str = "hooks/codexy-readiness-context.sh";
const PURPOSE_ROUTING_CONTEXT: u8 = 1;
const PURPOSE_READINESS_CONTEXT: u8 = 1 << 1;
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
    let mut hook_purposes = 0;
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
            hook_purposes |= check_group(&path, plugin_root, event, group)?;
        }
    }
    if hook_purposes & PURPOSE_ROUTING_CONTEXT == 0 {
        bail!(
            "{} {REQUIRED_EVENT} hook command must run {SESSION_START_SCRIPT}",
            display_relative(&path)
        );
    }
    if hook_purposes & PURPOSE_READINESS_CONTEXT == 0 {
        bail!(
            "{} {READINESS_EVENT} hook command must run {READINESS_SCRIPT}",
            display_relative(&path)
        );
    }
    for purpose in [
        lifecycle::PURPOSE_PR_TITLE_CHECK,
        lifecycle::PURPOSE_PR_LABEL_CHECK,
        lifecycle::PURPOSE_MERGE_MESSAGE_CHECK,
    ] {
        if hook_purposes & purpose == 0 {
            bail!(
                "{} {}",
                display_relative(&path),
                lifecycle::missing_hard_hook_message(purpose).unwrap_or("missing hard hook")
            );
        }
    }
    Ok(())
}

fn check_group(path: &Path, plugin_root: &Path, event: &str, group: &Value) -> Result<u8> {
    let object = group
        .as_object()
        .with_context(|| format!("{} {event} group must be an object", display_relative(path)))?;
    match object.get("matcher") {
        Some(value) => {
            let matcher = value.as_str();
            let Some(matcher) = matcher else {
                bail!(
                    "{} {event}.matcher must be a non-empty string when present",
                    display_relative(path)
                );
            };
            if event == REQUIRED_EVENT
                && !context::session_start_covers_resume_and_compact(path, Some(matcher))?
            {
                bail!(
                    "{} {REQUIRED_EVENT}.matcher must include resume and compact",
                    display_relative(path)
                );
            } else if event != REQUIRED_EVENT && matcher.trim().is_empty() {
                bail!(
                    "{} {event}.matcher must be a non-empty string when present",
                    display_relative(path)
                );
            }
        }
        None => {}
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
    let mut hook_purposes = 0;
    for handler in handlers {
        hook_purposes |= check_handler(path, plugin_root, event, handler)?;
    }
    Ok(hook_purposes)
}

fn check_handler(path: &Path, plugin_root: &Path, event: &str, handler: &Value) -> Result<u8> {
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
    let timeout = check_timeout(path, event, object)?;
    let mut hook_purpose = 0;
    if event == REQUIRED_EVENT && command_uses_script(command, SESSION_START_SCRIPT) {
        context::check_session_start_context(
            path,
            plugin_root,
            command,
            timeout,
            REQUIRED_EVENT,
            SESSION_START_SCRIPT,
        )?;
        hook_purpose |= PURPOSE_ROUTING_CONTEXT;
    }
    if event == READINESS_EVENT && command_uses_script(command, READINESS_SCRIPT) {
        context::check_readiness_context(path, plugin_root, command, timeout, READINESS_EVENT)?;
        hook_purpose |= PURPOSE_READINESS_CONTEXT;
    }
    hook_purpose |= lifecycle::check_hard_hook(path, plugin_root, event, command, timeout)?;
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
    Ok(hook_purpose)
}

fn command_uses_script(command: &str, script: &str) -> bool {
    let Some((hook_path, _)) = command::plugin_root_entrypoint_path(command) else {
        return false;
    };
    hook_path == Path::new(script)
}

fn check_timeout(path: &Path, event: &str, object: &serde_json::Map<String, Value>) -> Result<u64> {
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
    Ok(timeout)
}
