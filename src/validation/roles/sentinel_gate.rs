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
    "For review-feedback lanes, repeated-Codex-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MUST replay",
    "Every approval MUST reference the current diff or head, lane scope, touched implementation-file LOC evidence when applicable, verification commands and results, direct readback for structured files, reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
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
        .filter(|marker| !has_positive_marker(instructions, marker))
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

fn has_positive_marker(instructions: &str, marker: &str) -> bool {
    let mut search_start = 0;
    while let Some(relative_index) = instructions[search_start..].find(marker) {
        let marker_index = search_start + relative_index;
        if !is_prefix_negated(&instructions[..marker_index]) {
            return true;
        }
        search_start = marker_index + marker.len();
    }
    false
}

fn is_prefix_negated(prefix: &str) -> bool {
    let sentence_start = prefix
        .rfind(['.', '!', '?', '\n'])
        .map_or(0, |index| index + 1);
    let sentence_prefix = prefix[sentence_start..].to_ascii_lowercase();
    sentence_prefix.contains("must not")
        || sentence_prefix.contains("do not")
        || sentence_prefix.contains("should not")
}
