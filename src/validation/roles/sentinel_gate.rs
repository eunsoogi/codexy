use crate::paths::display_relative;
use std::path::Path;
use toml::Value;
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
const REASONING_CONTROL_EVIDENCE_FOLLOWUP_PREFIXES: &str = "this |that |it |evidence|requirement";
const REASONING_CONTROL_EVIDENCE_FOLLOWUP_REFERENCES: &str =
    "this evidence|that evidence|the evidence|this requirement|that requirement|the requirement|it";
const REASONING_CONTROL_PARAGRAPH_MARKERS: &[&str] = &[
    "reasoning control:",
    "packaged sentinel definition must run with the highest available reasoning setting",
    "model_reasoning_effort = \"xhigh\"",
    "reviewer evidence must record explicit unavailable evidence",
];
const REASONING_CONTROL_DISALLOWED_PATTERNS: &str = concat!(
    "absent reasoning control used or unavailable evidence|acceptable|aren't required|can be skipped|can include|can omit|can reference|does not have to|encouraged|",
    "does not need|does not require|doesn't have to|doesn't need|doesn't require|if applicable|if-applicable|if available|if needed|if possible|",
    "discretionary|do not have to|do not need|do not require|don't have to|don't need|don't require|",
    "forbidden|isn't needed|isn't necessary|isn't required|leave out|left out|",
    "may be ignored|may be skipped|may ignore|may include|may omit|may reference|may skip|missing|must attempt|must endeavor|must make reasonable efforts|must not|must prefer|must strive|must try|mustn't|",
    "need not|needn't|no need|",
    "no explicit reasoning control used or unavailable evidence|reasoning control used or unavailable evidence is absent|",
    "no reasoning control used or unavailable evidence|no requirement|not have to|",
    "not a requirement|not compulsory|not mandatory|not needed|not necessary|omitted|omit|optional|best effort|best-effort|",
    "permissive|prohibited|recommended|should|should include|should reference|skip|skipped|",
    "suggested|unnecessary|unless|waive|waived|waiver|as applicable|as-applicable|as needed|except if|except when|when-applicable|when available|when possible|where applicable|where-applicable|where available|where possible|where practical|without reasoning control used or unavailable evidence",
);
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
            let context = marker_context(&lower, start);
            contains_mandatory_reasoning_control_context(context)
                && !contains_disallowed_reasoning_control_context(context)
        })
}

fn has_negated_reasoning_control_evidence(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    lower
        .match_indices(REASONING_CONTROL_EVIDENCE_MARKER)
        .any(|(start, _)| has_disallowed_marker_context(&lower, start))
}
fn has_disallowed_marker_context(text: &str, marker_start: usize) -> bool {
    let context = marker_context(text, marker_start);
    contains_disallowed_reasoning_control_context(context)
        || context
            .split_once(REASONING_CONTROL_EVIDENCE_MARKER)
            .and_then(|(_, tail)| {
                tail.trim_start_matches(|ch| matches!(ch, ',' | ';') || ch.is_ascii_whitespace())
                    .split(|ch| ch == ',' || ch == ';')
                    .next()
            })
            .is_some_and(|tail| contains_context_pattern(tail, "when applicable"))
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

fn marker_context(text: &str, marker_start: usize) -> &str {
    let bytes = text.as_bytes();
    let mut start = marker_start;
    while start > 0 && bytes[start - 1] != b'.' {
        start -= 1;
    }
    let mut end = marker_start + REASONING_CONTROL_EVIDENCE_MARKER.len();
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
    let sentence = sentence.split('.').next().unwrap_or(sentence);
    let starts_with_followup = |candidate: &str| {
        REASONING_CONTROL_EVIDENCE_FOLLOWUP_PREFIXES
            .split('|')
            .any(|prefix| candidate.starts_with(prefix))
            || REASONING_CONTROL_DISALLOWED_PATTERNS
                .split('|')
                .any(|pattern| candidate.starts_with(pattern))
            || contains_required_negation(candidate)
    };
    starts_with_followup(sentence)
        || sentence
            .split_once(',')
            .is_some_and(|(_, tail)| starts_with_followup(tail.trim_start()))
        || sentence
            .split_once(' ')
            .is_some_and(|(_, tail)| starts_with_followup(tail.trim_start()))
        || (contains_disallowed_reasoning_control_context(sentence)
            && REASONING_CONTROL_EVIDENCE_FOLLOWUP_REFERENCES
                .split('|')
                .any(|pattern| contains_context_pattern(sentence, pattern)))
}

fn contains_disallowed_reasoning_control_context(clause: &str) -> bool {
    REASONING_CONTROL_DISALLOWED_PATTERNS
        .split('|')
        .any(|pattern| contains_context_pattern(clause, pattern))
        || contains_required_negation(clause)
}

fn contains_mandatory_reasoning_control_context(clause: &str) -> bool {
    contains_context_pattern(clause, "must")
        || (contains_context_pattern(clause, "required") && !contains_required_negation(clause))
}

fn contains_disallowed_reasoning_control_paragraph_context(paragraph: &str) -> bool {
    let after = paragraph
        .split_once("reasoning control:")
        .map(|(_, tail)| tail.trim_start());
    contains_context_pattern(paragraph, "negated")
        || paragraph.trim_start().starts_with("no reasoning control:")
        || after.is_some_and(|tail| tail.starts_with("no "))
}

fn contains_context_pattern(clause: &str, pattern: &str) -> bool {
    if pattern
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
    {
        return clause
            .split_ascii_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains(pattern);
    }
    clause
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| word == pattern)
}

fn contains_required_negation(clause: &str) -> bool {
    let words = clause
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    for (index, word) in words.iter().enumerate() {
        if *word != "required" {
            continue;
        }
        for negation_index in index.saturating_sub(8)..index {
            match words[negation_index] {
                "never" => return true,
                "not" => {
                    if words
                        .get(negation_index + 1)
                        .is_some_and(|word| matches!(*word, "only" | "just" | "merely" | "simply"))
                    {
                        continue;
                    }
                    return true;
                }
                "no" if words.get(negation_index + 1) == Some(&"longer") => return true,
                _ => {}
            }
        }
    }
    false
}
