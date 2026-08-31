use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

use super::routing_json;

const REQUEST_SCHEMA: &str = "codexy.tdd-classification-request.v1";
const ENGINEERING: [&str; 14] = [
    "production_code",
    "runtime_behavior",
    "validator",
    "parser",
    "markdown_backed_parser",
    "hook",
    "cli",
    "workflow",
    "installer",
    "package_resolution",
    "tool_behavior",
    "defect_repair",
    "behavior_preserving_refactor",
    "executable_contract",
];
const NON_ENGINEERING: [&str; 11] = [
    "readme",
    "documentation",
    "instruction_only_skill",
    "agent_prompt",
    "declarative_metadata",
    "issue_or_pr_metadata",
    "roadmap_or_release_prose",
    "inventory",
    "diagram",
    "example",
    "copy_edit",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    boundaries: Vec<String>,
}

pub(super) fn check(_plugin_root: &Path) -> Vec<String> {
    Vec::new()
}

pub(super) fn resolve(_plugin_root: &Path, text: &str) -> Result<Value> {
    let request = parse_request(text)?;
    if request.boundaries.iter().any(|boundary| {
        !ENGINEERING.contains(&boundary.as_str()) && !NON_ENGINEERING.contains(&boundary.as_str())
    }) {
        bail!("TDD classification request has an unrecognized boundary");
    }
    let engineering = request
        .boundaries
        .iter()
        .filter(|boundary| ENGINEERING.contains(&boundary.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let proportional = request
        .boundaries
        .iter()
        .filter(|boundary| NON_ENGINEERING.contains(&boundary.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let classification = match (engineering.is_empty(), proportional.is_empty()) {
        (false, true) => "engineering",
        (true, false) => "non_engineering",
        (false, false) => "mixed",
        (true, true) => bail!("TDD classification request has no recognized boundary"),
    };
    Ok(json!({
        "classification": classification,
        "engineering_tdd_required": !engineering.is_empty(),
        "tdd_boundaries": engineering,
        "proportional_proof_boundaries": proportional,
    }))
}

fn parse_request(text: &str) -> Result<Request> {
    let value = routing_json::parse(text).map_err(anyhow::Error::msg)?;
    let request = serde_json::from_value::<Request>(value)?;
    if request.schema != REQUEST_SCHEMA {
        bail!("TDD classification request has an unsupported schema");
    }
    if request.boundaries.is_empty() || has_duplicates(&request.boundaries) {
        bail!("TDD classification request must contain unique boundaries");
    }
    Ok(request)
}

fn has_duplicates(values: &[String]) -> bool {
    let unique = values.iter().collect::<std::collections::BTreeSet<_>>();
    unique.len() != values.len()
}
