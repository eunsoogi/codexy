use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

const REVIEWER_GATE_MARKERS: &[&str] = &[
    "validator/parser edge-case pass",
    "workflow/ownership compliance pass",
    "regression coverage and proof pass",
    "reasoning control",
    "reasoning control used or unavailable evidence",
    "unavailable evidence",
    "edge classes reviewed",
    "no-finding result",
    "repeated-Codex-feedback",
];

const REASONING_CONTROL_EVIDENCE_MARKER: &str = "reasoning control used or unavailable evidence";
const REASONING_CONTROL_EVIDENCE_FOLLOWUP_PREFIXES: &[&str] = &[
    "this ",
    "that ",
    "it ",
    "the evidence",
    "the requirement",
    "reviewer evidence",
    "evidence",
    "requirement",
];
const REASONING_CONTROL_PARAGRAPH_MARKERS: &[&str] = &[
    "reasoning control:",
    "packaged sentinel definition must run with the highest available reasoning setting",
    "model_reasoning_effort = \"xhigh\"",
    "reviewer evidence must record explicit unavailable evidence",
];
const REASONING_CONTROL_PARAGRAPH_DISALLOWED_PATTERNS: &[&str] = &["negated", "no"];
const REASONING_CONTROL_DISALLOWED_PATTERNS: &[&str] = &[
    "absent",
    "acceptable",
    "aren't required",
    "can be skipped",
    "can omit",
    "does not have to",
    "does not need",
    "does not require",
    "doesn't have to",
    "doesn't need",
    "doesn't require",
    "do not have to",
    "do not need",
    "do not require",
    "don't have to",
    "don't need",
    "don't require",
    "forbidden",
    "isn't needed",
    "isn't necessary",
    "isn't required",
    "leave out",
    "left out",
    "may be skipped",
    "may omit",
    "missing",
    "must not",
    "mustn't",
    "need not",
    "needn't",
    "no need",
    "no explicit reasoning control used or unavailable evidence",
    "no reasoning control used or unavailable evidence",
    "no requirement",
    "not have to",
    "not a requirement",
    "not be required",
    "not mandatory",
    "not needed",
    "not required",
    "not necessary",
    "omitted",
    "omit",
    "optional",
    "permissive",
    "prohibited",
    "skip",
    "skipped",
    "unnecessary",
    "waive",
    "waived",
    "waiver",
    "without",
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
    if !has_reasoning_control_paragraph(instructions) {
        errors.push(format!(
            "{} codexy-sentinel reasoning-control paragraph must be present and affirmative",
            display_relative(path)
        ));
    }
    if !has_affirmative_reasoning_control_evidence(instructions)
        || has_negated_reasoning_control_evidence(instructions)
    {
        errors.push(format!(
            "{} codexy-sentinel reasoning-control evidence must be affirmative and must not be negated, optional, waived, or permissive",
            display_relative(path)
        ));
    }
}

fn has_reasoning_control_paragraph(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    let Some(marker_start) = lower.find("reasoning control:") else {
        return false;
    };
    let paragraph = reasoning_control_paragraph(&lower, marker_start);
    REASONING_CONTROL_PARAGRAPH_MARKERS
        .iter()
        .all(|marker| paragraph.contains(marker))
        && !contains_disallowed_reasoning_control_context(paragraph)
        && !contains_disallowed_reasoning_control_paragraph_context(paragraph)
}

fn has_affirmative_reasoning_control_evidence(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    lower
        .match_indices(REASONING_CONTROL_EVIDENCE_MARKER)
        .any(|(start, _)| {
            !contains_disallowed_reasoning_control_context(marker_context(
                &lower,
                start,
                REASONING_CONTROL_EVIDENCE_MARKER.len(),
            ))
        })
}

fn has_negated_reasoning_control_evidence(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    lower
        .match_indices(REASONING_CONTROL_EVIDENCE_MARKER)
        .any(|(start, _)| {
            contains_disallowed_reasoning_control_context(marker_context(
                &lower,
                start,
                REASONING_CONTROL_EVIDENCE_MARKER.len(),
            ))
        })
}

fn reasoning_control_paragraph(text: &str, marker_start: usize) -> &str {
    let start = text[..marker_start]
        .rfind("\n\n")
        .map_or(0, |offset| offset + 2);
    let end = text[marker_start..]
        .find("\n\n")
        .map_or(text.len(), |offset| marker_start + offset);
    text[start..end].trim()
}

fn marker_context(text: &str, marker_start: usize, marker_len: usize) -> &str {
    let bytes = text.as_bytes();
    let mut start = marker_start;
    while start > 0 && bytes[start - 1] != b'.' {
        start -= 1;
    }
    let mut end = marker_start + marker_len;
    while end < bytes.len() && bytes[end] != b'.' {
        end += 1;
    }
    if let Some(next_start) = next_sentence_start(bytes, end) {
        let next_sentence = &text[next_start..];
        if has_reasoning_control_evidence_followup(next_sentence) {
            end = next_start;
            while end < bytes.len() && bytes[end] != b'.' {
                end += 1;
            }
        }
    }
    text[start..end].trim()
}

fn next_sentence_start(bytes: &[u8], clause_end: usize) -> Option<usize> {
    if clause_end >= bytes.len() || bytes[clause_end] != b'.' {
        return None;
    }
    let mut start = clause_end + 1;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    (start < bytes.len()).then_some(start)
}

fn has_reasoning_control_evidence_followup(sentence: &str) -> bool {
    REASONING_CONTROL_EVIDENCE_FOLLOWUP_PREFIXES
        .iter()
        .any(|prefix| sentence.starts_with(prefix))
}

fn contains_disallowed_reasoning_control_context(clause: &str) -> bool {
    REASONING_CONTROL_DISALLOWED_PATTERNS
        .iter()
        .any(|pattern| contains_context_pattern(clause, pattern))
}

fn contains_disallowed_reasoning_control_paragraph_context(paragraph: &str) -> bool {
    REASONING_CONTROL_PARAGRAPH_DISALLOWED_PATTERNS
        .iter()
        .any(|pattern| contains_context_pattern(paragraph, pattern))
}

fn contains_context_pattern(clause: &str, pattern: &str) -> bool {
    if pattern
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
    {
        return normalize_ascii_whitespace(clause).contains(&normalize_ascii_whitespace(pattern));
    }
    clause
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| word == pattern)
}

fn normalize_ascii_whitespace(text: &str) -> String {
    text.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}
