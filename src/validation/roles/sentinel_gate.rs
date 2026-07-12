use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

mod affirmative_clause;
mod reasoning_control;

const REVIEWER_GATE_MARKERS: &[&str] = &[
    "Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra.",
    "the reviewer evidence MUST record explicit unavailable evidence",
    "Reviewer specialization: MUST split the review into named passes",
    "The validator/parser edge-case pass MUST search",
    "The workflow/ownership compliance pass MUST verify",
    "The regression coverage and proof pass MUST verify",
    "For review-feedback lanes, repeated-Codex-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MUST replay",
    "Every approval MUST reference the current diff or head",
    "lane scope",
    "touched implementation-file LOC evidence when applicable",
    "verification commands and results",
    "direct readback for structured files",
    "reasoning control used or unavailable evidence",
    "direct reviewer passes performed",
    "edge classes reviewed",
    "replayed review examples when applicable",
    "no-finding result when no blockers remain",
    "any unresolved risk",
    "MUST identify formatting-only LOC remediation before approving readiness.",
    "MUST inspect the base-to-current reduction and block blank-line deletion or collapsed readable multiline code, tests, or instructions",
    "MUST permit a collapsed readable multiline construct when the same reduction includes independent structural remediation.",
];

const APPROVAL_EVIDENCE_MARKERS: &[&str] = &[
    "Every approval MUST reference the current diff or head",
    "lane scope",
    "touched implementation-file LOC evidence when applicable",
    "verification commands and results",
    "direct readback for structured files",
    "reasoning control used or unavailable evidence",
    "direct reviewer passes performed",
    "edge classes reviewed",
    "replayed review examples when applicable",
    "no-finding result when no blockers remain",
    "any unresolved risk",
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
    if !reasoning_control::has_reasoning_control_paragraph(instructions) {
        errors.push(format!(
            "{} codexy-sentinel reasoning-control paragraph must be present and affirmative",
            display_relative(path)
        ));
    }
    if !reasoning_control::has_affirmative_reasoning_control_evidence(instructions)
        || reasoning_control::has_negated_reasoning_control_evidence(instructions)
    {
        errors.push(format!(
            "{} codexy-sentinel reasoning-control evidence must be affirmative and must not be negated, optional, waived, or permissive",
            display_relative(path)
        ));
    }
    let missing_approval_markers = missing_approval_evidence_markers(instructions);
    if !missing_approval_markers.is_empty() {
        errors.push(format!(
            "{} codexy-sentinel approval evidence contract is missing: {}",
            display_relative(path),
            missing_approval_markers.join(", ")
        ));
    }
}

fn missing_approval_evidence_markers(instructions: &str) -> Vec<&'static str> {
    let contract_start = instructions.find("Evidence expectations:").unwrap_or(0);
    let Some(approval_start) = instructions[contract_start..]
        .find("Every approval MUST reference")
        .map(|index| contract_start + index)
    else {
        return APPROVAL_EVIDENCE_MARKERS.to_vec();
    };
    let sentence_end = instructions[approval_start..]
        .find(['.', '!', '?'])
        .map_or(instructions.len(), |index| approval_start + index);
    let sentence = &instructions[approval_start..sentence_end];
    APPROVAL_EVIDENCE_MARKERS
        .iter()
        .filter(|marker| !has_positive_marker(sentence, marker))
        .copied()
        .collect()
}

fn has_positive_marker(instructions: &str, marker: &str) -> bool {
    let mut found_positive = false;
    let mut search_start = 0;
    while let Some(relative_index) = instructions[search_start..].find(marker) {
        let marker_index = search_start + relative_index;
        if is_prefix_negated(instructions, marker_index, marker)
            || affirmative_clause::has_quoted_marker_prefix(
                &instructions[..marker_index],
                &instructions[marker_index + marker.len()..],
            )
            || is_marker_sentence_weakened(instructions, marker_index, marker)
        {
            return false;
        }
        found_positive = true;
        search_start = marker_index + marker.len();
    }
    found_positive
}

fn is_prefix_negated(instructions: &str, marker_index: usize, marker: &str) -> bool {
    let sentence_start = instructions[..marker_index]
        .rfind(['.', '!', '?'])
        .map_or(0, |index| index + 1);
    let prefix = instructions[sentence_start..marker_index].to_ascii_lowercase();
    let prefix = prefix.trim_end();
    contains_negated_language(prefix)
        && !has_mandatory_evidence_omission_prohibition_before_affirmative_reference(&format!(
            "{prefix}{}",
            marker.to_ascii_lowercase()
        ))
}

fn has_mandatory_evidence_omission_prohibition_before_affirmative_reference(
    sentence: &str,
) -> bool {
    ["must not omit", "must not skip", "must not leave out"]
        .iter()
        .filter_map(|prohibition| {
            sentence
                .rfind(prohibition)
                .map(|index| (index, prohibition))
        })
        .any(|(index, prohibition)| {
            let tail = &sentence[index + prohibition.len()..];
            tail.trim() == "reasoning control used or unavailable evidence"
                || tail.split_once(',').is_some_and(|(before, clause)| {
                    before.contains("reasoning control used or unavailable evidence")
                        && has_unweakened_approval_evidence_clause(clause)
                        && !contains_negated_language(&format!("{}{}", &sentence[..index], before))
                })
        })
}

fn has_unweakened_approval_evidence_clause(clause: &str) -> bool {
    let clause = clause
        .split_ascii_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let evidence = clause
        .strip_prefix("and must reference")
        .or_else(|| clause.strip_prefix("and must record"))
        .map_or("", |value| {
            value.trim_start().trim_start_matches([':', '-', ',', ';'])
        });
    APPROVAL_EVIDENCE_MARKERS
        .iter()
        .any(|marker| evidence.starts_with(&marker.to_ascii_lowercase()))
}

fn contains_negated_language(sentence: &str) -> bool {
    sentence.contains("must not")
        || sentence.contains("do not")
        || ["should not", "but not", "and not", "or not"]
            .iter()
            .any(|phrase| sentence.contains(phrase))
        || [" no", ", no", "no", " not", ", not", "not"]
            .iter()
            .any(|suffix| sentence.ends_with(suffix))
}

fn is_marker_sentence_weakened(instructions: &str, marker_index: usize, marker: &str) -> bool {
    let sentence_start = instructions[..marker_index]
        .rfind(['.', '!', '?'])
        .map_or(0, |index| index + 1);
    let marker_end = marker_index + marker.len();
    let sentence_end = instructions[marker_end..]
        .find(['.', '!', '?'])
        .map_or(instructions.len(), |index| marker_end + index);
    let sentence = instructions[sentence_start..sentence_end].to_ascii_lowercase();
    let marker_prefix = &sentence[..marker_index - sentence_start];
    let marker_tail_start = marker_index - sentence_start + marker.len();
    let marker_tail = &sentence[marker_tail_start..];
    sentence
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| matches!(word, "optional" | "permissive" | "waived"))
        || sentence.contains("not required")
        || sentence.contains("not mandatory")
        || sentence.contains("not needed")
        || affirmative_clause::has_weakened_marker_prefix(marker_prefix)
        || marker_tail_has_conditional_waiver(marker_tail)
        || sentence.contains("may skip")
        || sentence.contains("may omit")
        || sentence.contains("may ignore")
        || sentence.contains("may be skipped")
        || sentence.contains("may be omitted")
        || sentence.contains("may be ignored")
        || sentence.contains("can skip")
        || sentence.contains("can omit")
        || sentence.contains("can ignore")
        || sentence.contains("can be skipped")
        || sentence.contains("can be omitted")
        || sentence.contains("can be ignored")
}

fn marker_tail_has_conditional_waiver(tail: &str) -> bool {
    let tail = tail.trim_start_matches(|ch: char| {
        ch.is_ascii_whitespace() || matches!(ch, ':' | '-' | ',' | ';')
    });
    let clause_end = tail.find([',', ';']).unwrap_or(tail.len());
    let clause = tail[..clause_end].trim_start();
    [
        "if available",
        "when available",
        "if possible",
        "when possible",
        "if applicable",
        "when applicable",
        "as applicable",
        "where applicable",
        "if needed",
        "when needed",
        "as needed",
        "where needed",
    ]
    .iter()
    .any(|phrase| clause.starts_with(phrase))
}
