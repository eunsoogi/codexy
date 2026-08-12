use std::path::Path;

use anyhow::{Context as _, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::paths::display_relative;
use crate::validation::load_json;

const PATH: &str = "hooks/capability-contract.json";
const SCHEMA: &str = "codexy.hooks.capability-contract.v2";
const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

struct Expected {
    id: &'static str,
    trigger: &'static str,
    input: &'static str,
    launcher: &'static str,
    diagnostic: &'static str,
}

const CONCERNS: &[Expected] = &[Expected {
    id: "thread-delivery",
    trigger: "^codex_app__send_message_to_thread$",
    input: "codexy.hooks.thread-delivery.v1",
    launcher: "codexy-thread-delivery",
    diagnostic: "CODEXY_THREAD_DELIVERY_",
}];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CapabilityContract {
    schema: String,
    content_digest: String,
    concerns: Vec<Concern>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Concern {
    concern_id: String,
    trigger: String,
    events: Vec<String>,
    input_contract: String,
    preventive: bool,
    entrypoints: Vec<String>,
    diagnostic_family: String,
    content_digest: String,
}

pub(super) fn check(plugin_root: &Path) -> Result<()> {
    let path = plugin_root.join(PATH);
    let contract: CapabilityContract =
        serde_json::from_value(load_json(&path)?).map_err(|error| {
            anyhow!(
                "{} must match hook capability contract schema: {error}",
                display_relative(&path)
            )
        })?;
    if contract.schema != SCHEMA || contract.concerns.len() != CONCERNS.len() {
        bail!(
            "{} must contain the exact ordered hook concerns",
            display_relative(&path)
        );
    }
    for (actual, expected) in contract.concerns.iter().zip(CONCERNS) {
        let entrypoints = entrypoints(expected.launcher);
        if actual.concern_id != expected.id
            || actual.trigger != expected.trigger
            || actual.events.iter().map(String::as_str).collect::<Vec<_>>() != EVENTS
            || actual.input_contract != expected.input
            || !actual.preventive
            || actual.entrypoints != entrypoints
            || actual.diagnostic_family != expected.diagnostic
            || actual.content_digest != concern_digest(expected)
        {
            bail!(
                "{} has a missing, extra, stale, or tampered concern: {}",
                display_relative(&path),
                expected.id
            );
        }
    }
    if contract.content_digest != contract_digest() {
        bail!(
            "{} content digest does not bind its exact concerns",
            display_relative(&path)
        );
    }
    Ok(())
}

pub(super) fn check_topology(path: &Path, events: &Map<String, Value>) -> Result<()> {
    if events.len() != EVENTS.len() || EVENTS.iter().any(|event| !events.contains_key(*event)) {
        bail!(
            "{} must configure only the two preventive concern events",
            display_relative(path)
        );
    }
    for event in EVENTS {
        let groups = events[*event].as_array().with_context(|| {
            format!("{} {event} groups must be an array", display_relative(path))
        })?;
        if groups.len() != CONCERNS.len() {
            bail!(
                "{} {event} must bind every concern exactly once",
                display_relative(path)
            );
        }
        for (group, concern) in groups.iter().zip(CONCERNS) {
            let object = group
                .as_object()
                .context("concern group must be an object")?;
            let handlers = object.get("hooks").and_then(Value::as_array);
            if object.get("matcher").and_then(Value::as_str) != Some(concern.trigger)
                || handlers.is_none_or(|items| items.len() != 1)
            {
                bail!(
                    "{} {event} concern topology mismatch: {}",
                    display_relative(path),
                    concern.id
                );
            }
            let handler = handlers
                .and_then(|items| items[0].as_object())
                .context("handler")?;
            let command = format!("\"${{PLUGIN_ROOT}}/hooks/{}.sh\" {event}", concern.launcher);
            let windows = format!(
                "\"${{PLUGIN_ROOT}}/hooks/{}.cmd\" {event}",
                concern.launcher
            );
            if handler.get("command").and_then(Value::as_str) != Some(command.as_str())
                || handler.get("commandWindows").and_then(Value::as_str) != Some(windows.as_str())
            {
                bail!(
                    "{} {event} concern entrypoint mismatch: {}",
                    display_relative(path),
                    concern.id
                );
            }
        }
    }
    Ok(())
}

fn entrypoints(launcher: &str) -> Vec<String> {
    ["sh", "cmd", "py"]
        .map(|extension| format!("{launcher}.{extension}"))
        .to_vec()
}

fn concern_digest(concern: &Expected) -> String {
    fnv(&[
        concern.id,
        concern.trigger,
        concern.input,
        concern.diagnostic,
        &EVENTS.join("\u{1f}"),
        &entrypoints(concern.launcher).join("\u{1f}"),
    ]
    .join("\0"))
}

fn contract_digest() -> String {
    let digests = CONCERNS.iter().map(concern_digest).collect::<Vec<_>>();
    fnv(&[SCHEMA, &digests.join("\0")].join("\0"))
}

fn fnv(value: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}
