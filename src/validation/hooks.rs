mod admission_artifact;
mod agent_update_safety;
mod command;
#[allow(dead_code)]
mod context;
mod model;
mod policy_inventory;
mod policy_inventory_discovery;
mod policy_inventory_frontmatter;
mod post_compact;
mod safety;

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::{Map, Value};

use crate::paths::display_relative;
use crate::validation::load_json;

const HOOKS_PATH: &str = "hooks/hooks.json";
const ADMISSION_SCRIPT: &str = "hooks/codexy-admission.sh";
const ADMISSION_WINDOWS_SCRIPT: &str = "hooks/codexy-admission.cmd";
const REQUIRED_BINDINGS: [model::HookBinding; 2] = [
    model::HookBinding {
        event: model::HookEvent::PreToolUse,
        purpose: model::HookPurpose::Admission,
    },
    model::HookBinding {
        event: model::HookEvent::PermissionRequest,
        purpose: model::HookPurpose::Admission,
    },
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    match check_inner(plugin_root) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

pub fn policy_inventory_discovery_json(plugin_root: &Path) -> Result<String> {
    Ok(serde_json::to_string(
        &policy_inventory_discovery::discover(plugin_root)?,
    )?)
}

fn check_inner(plugin_root: &Path) -> Result<()> {
    admission_artifact::check(plugin_root)?;
    policy_inventory::check(plugin_root)?;
    let selected_context_event = post_compact::check(plugin_root)?;
    let path = plugin_root.join(HOOKS_PATH);
    let data = load_json(&path)?;
    let events = data
        .get("hooks")
        .and_then(Value::as_object)
        .with_context(|| format!("{} hooks must be an object", display_relative(&path)))?;
    post_compact::check_topology(&path, events, selected_context_event)?;
    check_event_set(&path, events, selected_context_event)?;
    for binding in REQUIRED_BINDINGS {
        match binding.purpose {
            model::HookPurpose::Admission => {
                check_admission_event(&path, plugin_root, events, binding.event)?;
            }
        }
    }
    Ok(())
}

fn check_event_set(
    path: &Path,
    events: &Map<String, Value>,
    selected_context_event: post_compact::ContextEvent,
) -> Result<()> {
    let actual = events
        .keys()
        .map(|event| model::HookEvent::parse(event))
        .collect::<Result<BTreeSet<_>>>()?;
    let mut expected = REQUIRED_BINDINGS
        .iter()
        .map(|binding| binding.event)
        .collect::<BTreeSet<_>>();
    if selected_context_event != post_compact::ContextEvent::None {
        expected.insert(model::HookEvent::parse(selected_context_event.as_str())?);
    }
    if actual != expected {
        bail!(
            "{} must define only event-native PreToolUse and PermissionRequest enforcement{}",
            display_relative(path),
            if selected_context_event != post_compact::ContextEvent::None {
                " plus one proven model-context delivery"
            } else {
                ""
            }
        );
    }
    Ok(())
}

fn check_admission_event(
    path: &Path,
    plugin_root: &Path,
    events: &Map<String, Value>,
    event: model::HookEvent,
) -> Result<()> {
    let name = event.as_str();
    let groups = events
        .get(name)
        .and_then(Value::as_array)
        .filter(|groups| groups.len() == 1)
        .with_context(|| {
            format!(
                "{} {name} must have exactly one matcher group",
                display_relative(path)
            )
        })?;
    let group = groups[0]
        .as_object()
        .with_context(|| format!("{} {name} group must be an object", display_relative(path)))?;
    check_matcher(path, name, group)?;
    let handlers = group
        .get("hooks")
        .and_then(Value::as_array)
        .filter(|handlers| handlers.len() == 1)
        .with_context(|| {
            format!(
                "{} {name} must have exactly one handler",
                display_relative(path)
            )
        })?;
    check_admission_handler(path, plugin_root, name, &handlers[0])
}

fn check_matcher(path: &Path, event: &str, group: &Map<String, Value>) -> Result<()> {
    let matcher = group
        .get("matcher")
        .and_then(Value::as_str)
        .filter(|matcher| !matcher.trim().is_empty())
        .with_context(|| {
            format!(
                "{} {event}.matcher must be a non-empty string",
                display_relative(path)
            )
        })?;
    if matcher != "*" {
        bail!(
            "{} {event}.matcher must be \"*\" so the policy runtime decides no-op versus deny",
            display_relative(path)
        );
    }
    Ok(())
}

fn check_admission_handler(
    path: &Path,
    plugin_root: &Path,
    event: &str,
    handler: &Value,
) -> Result<()> {
    let object = handler.as_object().with_context(|| {
        format!(
            "{} {event} handler must be an object",
            display_relative(path)
        )
    })?;
    if object.get("type").and_then(Value::as_str) != Some("command") {
        bail!(
            "{} {event} handler must use type \"command\"",
            display_relative(path)
        );
    }
    if object
        .get("async")
        .is_some_and(|value| value != &Value::Bool(false))
    {
        bail!(
            "{} {event} handler MUST NOT run asynchronously",
            display_relative(path)
        );
    }
    let command_text = object
        .get("command")
        .and_then(Value::as_str)
        .with_context(|| {
            format!(
                "{} {event} command must be a string",
                display_relative(path)
            )
        })?;
    command::check_command(path, plugin_root, event, command_text)?;
    let expected = format!("\"${{PLUGIN_ROOT}}/{ADMISSION_SCRIPT}\" {event}");
    if command_text != expected {
        bail!(
            "{} {event} must invoke the single admission dispatcher exactly",
            display_relative(path)
        );
    }
    let windows_command = object
        .get("commandWindows")
        .and_then(Value::as_str)
        .with_context(|| {
            format!(
                "{} {event} commandWindows must be a string",
                display_relative(path)
            )
        })?;
    command::check_command(path, plugin_root, event, windows_command)?;
    let expected_windows = format!("\"${{PLUGIN_ROOT}}/{ADMISSION_WINDOWS_SCRIPT}\" {event}");
    if windows_command != expected_windows {
        bail!(
            "{} {event} commandWindows must invoke the single CMD admission dispatcher exactly",
            display_relative(path)
        );
    }
    check_timeout(path, event, object)?;
    if object.get("statusMessage").is_some_and(|value| {
        !value
            .as_str()
            .is_some_and(|message| !message.trim().is_empty())
    }) {
        bail!(
            "{} {event} statusMessage must be a non-empty string",
            display_relative(path)
        );
    }
    Ok(())
}

fn check_timeout(path: &Path, event: &str, object: &Map<String, Value>) -> Result<()> {
    object
        .get("timeout")
        .and_then(Value::as_u64)
        .filter(|timeout| *timeout == 5)
        .with_context(|| {
            format!(
                "{} {event} timeout must be exactly 5 seconds",
                display_relative(path)
            )
        })?;
    Ok(())
}
