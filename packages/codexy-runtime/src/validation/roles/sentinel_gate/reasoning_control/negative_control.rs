use super::{DISALLOWED_PATTERNS, MANDATORY_EVIDENCE_OMISSION_PROHIBITIONS};

pub(super) fn contains_disallowed_marker_context(context: &str) -> bool {
    if !has_mandatory_evidence_omission_prohibition(context) {
        return contains_disallowed_context(context);
    }
    let normalized = MANDATORY_EVIDENCE_OMISSION_PROHIBITIONS
        .iter()
        .fold(context.to_owned(), |context, prohibition| {
            context.replace(prohibition, "")
        });
    contains_disallowed_context(&normalized)
}

pub(super) fn contains_disallowed_context(clause: &str) -> bool {
    DISALLOWED_PATTERNS
        .split('|')
        .any(|pattern| contains_context_pattern(clause, pattern))
        || contains_required_negation(clause)
}

pub(super) fn references_reasoning_evidence_requirement(clause: &str) -> bool {
    contains_context_pattern(clause, "reasoning control")
        || contains_context_pattern(clause, "reasoning control evidence")
        || (contains_context_pattern(clause, "reviewer")
            && contains_context_pattern(clause, "evidence"))
}

pub(super) fn contains_mandatory_context(clause: &str) -> bool {
    has_mandatory_evidence_omission_prohibition(clause)
        || ("reference|record"
            .split('|')
            .any(|pattern| contains_context_pattern(clause, pattern))
            && (contains_context_pattern(clause, "must")
                || (contains_context_pattern(clause, "required")
                    && !contains_required_negation(clause))))
}

fn has_mandatory_evidence_omission_prohibition(clause: &str) -> bool {
    MANDATORY_EVIDENCE_OMISSION_PROHIBITIONS
        .iter()
        .any(|prohibition| contains_context_pattern(clause, prohibition))
}

pub(super) fn contains_disallowed_paragraph_context(paragraph: &str) -> bool {
    contains_context_pattern(paragraph, "negated")
        || paragraph.trim_start().starts_with("no reasoning control:")
        || paragraph.trim_start().starts_with("not reasoning control:")
        || paragraph
            .split_once("reasoning control:")
            .is_some_and(|(_, tail)| tail.trim_start().starts_with("no "))
}

pub(super) fn contains_scoped_opt_out(clause: &str) -> bool {
    let words = context_words(clause);
    let inline_condition = words
        .iter()
        .position(|word| {
            matches!(
                *word,
                "if" | "when" | "whenever" | "where" | "unless" | "provided"
            )
        })
        .is_some_and(|index| {
            index > 0
                && words[..index]
                    .iter()
                    .any(|word| matches!(*word, "must" | "required"))
                && words[index + 1..]
                    .iter()
                    .any(|word| matches!(*word, "reference" | "record"))
        });
    words.last() == Some(&"not")
        || inline_condition
        || words.first().is_some_and(|word| {
            matches!(
                *word,
                "if" | "when" | "whenever" | "where" | "unless" | "provided"
            )
        })
        || "required if|required when|required whenever|required where|required unless|required provided|required only if|required only when|required only whenever|required only where|required only unless|required only provided"
            .split('|')
            .any(|pattern| contains_context_pattern(clause, pattern))
        || "except|except in|except for|only for|only if|only when"
            .split('|')
            .any(|pattern| contains_context_pattern(clause, pattern))
}

pub(super) fn contains_context_pattern(clause: &str, pattern: &str) -> bool {
    if pattern
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
    {
        let clause_words = context_words(clause);
        let pattern_words = context_words(pattern);
        if pattern_words.is_empty() || pattern_words.len() > clause_words.len() {
            return false;
        }
        return clause_words
            .windows(pattern_words.len())
            .any(|window| window == pattern_words.as_slice());
    }
    clause
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| word == pattern)
}

fn context_words(text: &str) -> Vec<&str> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|word| !word.is_empty())
        .collect()
}

pub(super) fn contains_required_negation(clause: &str) -> bool {
    let words = context_words(clause);
    words.iter().enumerate().any(|(index, word)| {
        *word == "required"
            && (index.saturating_sub(8)..index)
                .chain(index + 1..(index + 6).min(words.len()))
                .any(|negation_index| is_required_negation(&words, negation_index))
    })
}

fn is_required_negation(words: &[&str], index: usize) -> bool {
    match words[index] {
        "never" => true,
        "not" => !words
            .get(index + 1)
            .is_some_and(|word| matches!(*word, "only" | "just" | "merely" | "simply")),
        "isn" | "aren" | "wasn" | "weren" | "doesn" | "don" | "didn" | "needn" => {
            words.get(index + 1) == Some(&"t")
        }
        "no" => words.get(index + 1) == Some(&"longer"),
        _ => false,
    }
}
