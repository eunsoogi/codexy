use std::path::Path;

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

use crate::paths::display_relative;
use crate::validation::load_json;

const PATH: &str = "hooks/capability-contract.json";
const SCHEMA: &str = "codexy.hooks.capability-contract";
const CAPABILITIES: &[(&str, &str, bool, &[&str])] = &[
    (
        "PermissionRequest",
        "codexy.hooks.permission-request",
        true,
        &[],
    ),
    (
        "PreToolUse",
        "codexy.hooks.pre-tool-use",
        true,
        &[
            "bash-command",
            "github-title",
            "thread-delivery-model-thinking",
        ],
    ),
    ("SessionEnd", "codexy.hooks.session-end", false, &[]),
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CapabilityContract {
    schema: String,
    content_digest: String,
    capabilities: Vec<Capability>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Capability {
    event: String,
    schema: String,
    authoritative_inputs: Vec<String>,
    preventive: bool,
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
    if contract.schema != SCHEMA || contract.capabilities.len() != CAPABILITIES.len() {
        bail!(
            "{} must contain the exact checked-in hook capability schemas",
            display_relative(&path)
        );
    }
    for (actual, expected) in contract.capabilities.iter().zip(CAPABILITIES) {
        let (event, schema, preventive, inputs) = expected;
        if actual.event != *event
            || actual.schema != *schema
            || actual.preventive != *preventive
            || actual
                .authoritative_inputs
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != *inputs
            || actual.content_digest != capability_digest(event, schema, *preventive, inputs)
        {
            bail!(
                "{} has a missing, extra, stale, or tampered capability",
                display_relative(&path)
            );
        }
    }
    if contract.content_digest != contract_digest() {
        bail!(
            "{} content digest does not bind its exact capabilities",
            display_relative(&path)
        );
    }
    Ok(())
}

fn capability_digest(event: &str, schema: &str, preventive: bool, inputs: &[&str]) -> String {
    let semantics = if preventive {
        "preventive"
    } else {
        "nonpreventive"
    };
    fnv(&[event, schema, semantics, &inputs.join("\u{1f}")].join("\0"))
}

fn contract_digest() -> String {
    let digests = CAPABILITIES
        .iter()
        .map(|(event, schema, preventive, inputs)| {
            capability_digest(event, schema, *preventive, inputs)
        })
        .collect::<Vec<_>>();
    fnv(&[SCHEMA, &digests.join("\0")].join("\0"))
}

fn fnv(value: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}
