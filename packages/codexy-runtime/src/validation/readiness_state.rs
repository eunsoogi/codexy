#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ReadinessState {
    Neutral,
    Affirmative,
}

const NEUTRAL_WORDS: &[&str] = &[
    "blocked",
    "blocking",
    "waiting",
    "pending",
    "unresolved",
    "incomplete",
];
const AFFIRMATIVE_WORDS: &[&str] = &[
    "ready",
    "complete",
    "completed",
    "passed",
    "clean",
    "none",
    "no",
];

pub(crate) fn classify(value: &str) -> Option<ReadinessState> {
    let words = words(value);
    let first = words.first()?;
    if has_negated_completion(&words) || NEUTRAL_WORDS.contains(first) {
        return Some(ReadinessState::Neutral);
    }
    if AFFIRMATIVE_WORDS.contains(first) {
        return Some(ReadinessState::Affirmative);
    }
    words
        .iter()
        .any(|word| NEUTRAL_WORDS.contains(word))
        .then_some(ReadinessState::Neutral)
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
