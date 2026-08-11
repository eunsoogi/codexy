const READINESS_WORDS: [&str; 2] = ["pr", "merge"];
use crate::validation::readiness_state::{ReadinessField, ReadinessState, classify};

pub(crate) fn is_neutral_heading(fragment: &str) -> bool {
    let Some((heading, value)) = fragment.trim().split_once(':') else {
        return false;
    };
    let heading_words = words(heading);
    heading_field(&heading_words).is_some_and(|field| is_pending_status(value, field))
}

pub(crate) fn is_neutral_span_at(text: &str, marker_start: usize) -> bool {
    let line_start = text[..marker_start]
        .rfind(['.', '\n'])
        .map_or(0, |index| index + 1);
    let line = &text[line_start..];
    let Some((heading, value)) = line.split_once(':') else {
        return false;
    };
    let heading_end = line_start + heading.len();
    if marker_start > heading_end {
        return false;
    }
    let value = if value.trim().is_empty() {
        line.split_once('\n').map_or(value, |(_, next)| next)
    } else {
        value
    };
    heading_field(&words(heading)).is_some_and(|field| is_pending_status(value, field))
}

fn heading_field(words: &[&str]) -> Option<ReadinessField> {
    let Some(readiness) = words.iter().position(|word| *word == "readiness") else {
        return ready_alias_field(words);
    };
    let owner = readiness.checked_sub(1).and_then(|index| words.get(index));
    (matches!(owner, Some(owner) if READINESS_WORDS.contains(owner))).then(|| {
        if words[readiness + 1..].contains(&"blocker")
            || words[readiness + 1..].contains(&"blockers")
        {
            ReadinessField::Blocker
        } else if words[readiness + 1..].contains(&"status") {
            ReadinessField::Status
        } else {
            ReadinessField::Claim
        }
    })
}

fn ready_alias_field(words: &[&str]) -> Option<ReadinessField> {
    words
        .windows(2)
        .any(|pair| pair == ["pr", "ready"] || pair == ["merge", "ready"])
        .then_some(ReadinessField::Claim)
}

fn is_pending_status(value: &str, field: ReadinessField) -> bool {
    matches!(classify(value, field), Some(ReadinessState::Neutral))
}

fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}
