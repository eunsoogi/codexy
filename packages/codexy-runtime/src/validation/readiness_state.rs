#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ReadinessState {
    Neutral,
    Affirmative,
}

#[derive(Clone, Copy)]
pub(crate) enum ReadinessField {
    Claim,
    Blocker,
    Status,
}

const NEUTRAL_WORDS: &[&str] = &[
    "blocked",
    "blocking",
    "waiting",
    "pending",
    "unresolved",
    "incomplete",
];
const AFFIRMATIVE_WORDS: &[&str] = &["ready", "complete", "completed", "passed", "clean"];

pub(crate) fn classify(value: &str, field: ReadinessField) -> Option<ReadinessState> {
    if has_negative_prefix(value) {
        return Some(ReadinessState::Neutral);
    }
    let words = words(value);
    let first = words.first()?;
    if has_negated_completion(&words) || NEUTRAL_WORDS.contains(first) {
        return Some(ReadinessState::Neutral);
    }
    if matches!(field, ReadinessField::Blocker) && matches!(*first, "none" | "no" | "clear") {
        return Some(ReadinessState::Affirmative);
    }
    if AFFIRMATIVE_WORDS.contains(first) {
        return Some(ReadinessState::Affirmative);
    }
    words
        .iter()
        .any(|word| NEUTRAL_WORDS.contains(word))
        .then_some(ReadinessState::Neutral)
}

fn has_negative_prefix(value: &str) -> bool {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']);
    [
        "no", "false", "not", "isn't", "aren't", "missing", "absent", "n/a",
    ]
    .iter()
    .any(|prefix| value.strip_prefix(prefix).is_some_and(has_boundary))
}

fn has_boundary(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn has_negated_completion(words: &[&str]) -> bool {
    words
        .windows(2)
        .any(|pair| pair[0] == "not" && matches!(pair[1], "ready" | "complete"))
        || words
            .windows(3)
            .any(|pair| pair[0] == "not" && matches!(pair[2], "ready" | "complete"))
}

fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}
