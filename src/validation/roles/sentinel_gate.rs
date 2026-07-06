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
const REASONING_CONTROL_EVIDENCE_FOLLOWUP_REFERENCES: &str = "this evidence|that evidence|the evidence|this requirement|that requirement|the requirement|this|that|it";
const REASONING_CONTROL_PARAGRAPH_MARKERS: &[&str] = &[
    "reasoning control:",
    "packaged sentinel definition must run with the highest available reasoning setting",
    "model_reasoning_effort = \"xhigh\"",
    "reviewer evidence must record explicit unavailable evidence",
];
const REASONING_CONTROL_DISALLOWED_PATTERNS: &str = concat!(
    "absent reasoning control used or unavailable evidence|acceptable|allowed to disregard|allowed to ignore|aren't required|can be disregarded|can be ignored|can be skipped|can decide whether|can choose whether|can disregard|can ignore|can include|can omit|can reference|consider|considered|does not have to|encouraged|does not need|does not require|doesn't have to|doesn't need|doesn't require|if applicable|if-applicable|if available|if feasible|if needed|if possible|",
    "discretionary|do not have to|do not need|do not require|don't have to|don't need|don't require|reviewer discretion|choose not|for awareness only|forbidden|isn't needed|isn't necessary|isn't required|leave it out|leave out|left out|may be disregarded|may be ignored|may be skipped|may disregard|may ignore|may include|may omit|may reference|may skip|missing reasoning control used or unavailable evidence|must attempt|must endeavor|must evaluate|must inspect|must make reasonable efforts|must never|must not|must-not|must prefer|must review|must strive|must try|mustn't|need not|needn't|no need|no explicit reasoning control used or unavailable evidence|reasoning control used or unavailable evidence is absent|required to evaluate|required to inspect|required to review|",
    "no reasoning control used or unavailable evidence|no requirement|not have to|not a requirement|not binding|not compulsory|not expected|not mandatory|not obligatory|not needed|not necessary|omitted|omit|optional|best effort|best-effort|only if requested|ought|permissive|permitted to disregard|permitted to ignore|prohibited|provided that|recommended|reviewer choice|should|should include|should reference|skip|skipped|suggested|subject to tool availability|unnecessary|unless|up to the reviewer|voluntary|waive|waived|waiver|advisable|as applicable|as-applicable|as appropriate|as needed|except if|except when|reviewer's discretion|when applicable|when-applicable|when available|when feasible|when needed|when possible|where applicable|where-applicable|where available|where needed|where possible|where practical|without reasoning control used or unavailable evidence",
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
                && !contains_disallowed_marker_scoped_context(context)
        })
}
fn has_negated_reasoning_control_evidence(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    lower
        .match_indices(REASONING_CONTROL_EVIDENCE_MARKER)
        .any(|(start, _)| contains_disallowed_marker_scoped_context(marker_context(&lower, start)))
}
fn contains_disallowed_marker_scoped_context(context: &str) -> bool {
    let Some((head, tail)) = context.split_once(REASONING_CONTROL_EVIDENCE_MARKER) else {
        return contains_disallowed_reasoning_control_context(context);
    };
    let head_segments = head.split([',', ';']).map(str::trim).collect::<Vec<_>>();
    let preamble = head_segments.first().copied().unwrap_or(head);
    if contains_disallowed_reasoning_control_context(preamble)
        || head_segments
            .iter()
            .rev()
            .skip(1)
            .take(1)
            .any(|segment| contains_disallowed_reasoning_control_context(segment))
        || "if applicable, reference|when applicable, reference|where applicable, reference|as applicable, reference|reference, if applicable|reference, when applicable|reference, where applicable|reference, as applicable|reference if applicable|reference when applicable|reference where applicable|reference as applicable"
            .split('|')
            .any(|pattern| contains_context_pattern(head, pattern))
    {
        return true;
    }
    let scoped_head = head.rsplit([',', ';']).next().unwrap_or(head);
    let sentence_end = tail.find('.').unwrap_or(tail.len());
    let sentence_tail = &tail[..sentence_end];
    let mut tail_segments = sentence_tail.split([',', ';']);
    let scoped_tail = tail_segments.next().unwrap_or(sentence_tail);
    let opt_out_tail = tail_segments
        .filter(|segment| has_reasoning_control_evidence_followup(segment.trim_start()))
        .collect::<Vec<_>>()
        .join(" ");
    let followups = &tail[sentence_end..];
    contains_disallowed_reasoning_control_context(&format!(
        "{scoped_head}{REASONING_CONTROL_EVIDENCE_MARKER}{scoped_tail} {opt_out_tail}{followups}"
    ))
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
    while let Some(next_start) = next_sentence_start(bytes, end) {
        let next_sentence = &text[next_start..];
        if !has_reasoning_control_evidence_followup(next_sentence) {
            break;
        }
        end = next_start;
        while end < bytes.len() && bytes[end] != b'.' {
            end += 1;
        }
    }
    text[start..end].trim()
}
fn next_sentence_start(bytes: &[u8], clause_end: usize) -> Option<usize> {
    (clause_end < bytes.len() && bytes[clause_end] == b'.').then_some(())?;
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
    "reference|record"
        .split('|')
        .any(|pattern| contains_context_pattern(clause, pattern))
        && (contains_context_pattern(clause, "must")
            || (contains_context_pattern(clause, "required")
                && !contains_required_negation(clause)))
}
fn contains_disallowed_reasoning_control_paragraph_context(paragraph: &str) -> bool {
    contains_context_pattern(paragraph, "negated")
        || paragraph.trim_start().starts_with("no reasoning control:")
        || paragraph
            .split_once("reasoning control:")
            .is_some_and(|(_, tail)| tail.trim_start().starts_with("no "))
}
fn contains_context_pattern(clause: &str, pattern: &str) -> bool {
    if pattern
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
    {
        let words = clause.split_ascii_whitespace().collect::<Vec<_>>();
        return words.join(" ").contains(pattern);
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
