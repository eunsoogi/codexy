use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

const REVIEWER_GATE_MARKERS: &[&str] = &[
    "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting",
    "the reviewer evidence MUST record explicit unavailable evidence",
    "Reviewer specialization: MUST split the review into named passes",
    "The validator/parser edge-case pass MUST search",
    "The workflow/ownership compliance pass MUST verify",
    "The regression coverage and proof pass MUST verify",
    "edge classes reviewed",
    "no-finding result",
    "repeated-Codex-feedback",
];

pub(super) fn check(path: &Path, agent: &Value, errors: &mut Vec<String>) {
    if agent.get("model_reasoning_effort").and_then(Value::as_str) != Some("xhigh") {
        errors.push(format!(
            "{} codexy-sentinel model_reasoning_effort must be xhigh",
            display_relative(path)
        ));
    }

    let instructions = agent
        .get("developer_instructions")
        .and_then(Value::as_str)
        .unwrap_or("");
    let missing_markers = REVIEWER_GATE_MARKERS
        .iter()
        .filter(|marker| !instructions.contains(**marker))
        .copied()
        .collect::<Vec<_>>();
    if !missing_markers.is_empty() {
        errors.push(format!(
            "{} codexy-sentinel reviewer gate contract is missing: {}",
            display_relative(path),
            missing_markers.join(", ")
        ));
    }
}
