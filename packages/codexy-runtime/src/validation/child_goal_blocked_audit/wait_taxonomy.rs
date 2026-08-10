#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::validation) enum WaitDisposition {
    Nonterminal,
    Actionable,
}

const NONTERMINAL_PRODUCERS: &[&str] = &[
    "sentinel-running",
    "child-pending",
    "ci-queued",
    "connector-review-pending",
    "reviewer-pending",
    "parent-authorization-pending",
    "dependency-integration-pending",
    "resource-slot-pending",
    "alternate-evidence-pending",
    "async-tool-pending",
    "event-idle-child",
];

const REVIEW_SUBJECTS: &[&str] = &[
    "review",
    "reviewer",
    "review feedback",
    "review comment",
    "requested changes",
    "changes requested",
    "maintainer feedback",
    "feedback from maintainer",
    "security review",
];

const ACTIONABLE_REVIEW_STATES: &[&str] = &[
    "actionable feedback",
    "actionable review",
    "changes requested",
    "requested changes",
    "review suggestion",
    "unresolved review",
    "resolution required",
];

const PENDING_STATES: &[&str] = &[
    "pending",
    "waiting",
    "awaiting",
    "in progress",
    "processing",
    "not returned",
    "not yet returned",
    "has not returned",
];

const OPERATIONAL_NONTERMINAL_STATES: &[&str] = &[
    "hard",
    "slow",
    "uncertain",
    "uncertainty",
    "incomplete",
    "token pressure",
    "token budget",
    "token limit",
];

const NEGATIONS: &[&str] = &["no", "not", "none", "without", "neither"];
const RESOLUTIONS: &[&str] = &[
    "resolved",
    "fixed",
    "addressed",
    "cleared",
    "complete",
    "completed",
];

pub(in crate::validation) fn classify_producer(value: &str) -> Option<WaitDisposition> {
    NONTERMINAL_PRODUCERS
        .contains(&value)
        .then_some(WaitDisposition::Nonterminal)
}

pub(in crate::validation) fn classify_wait_text(text: &str) -> Option<WaitDisposition> {
    let words = words(text);
    if contains_any_phrase(&words, OPERATIONAL_NONTERMINAL_STATES) {
        return Some(WaitDisposition::Nonterminal);
    }
    classify_reviewer_words(&words)
}

pub(in crate::validation) fn classify_reviewer_text(text: &str) -> Option<WaitDisposition> {
    classify_reviewer_words(&words(text))
}

fn classify_reviewer_words(words: &[&str]) -> Option<WaitDisposition> {
    if !contains_any_phrase(&words, REVIEW_SUBJECTS)
        && !has_nearby_words(&words, "feedback", "maintainer", 3)
    {
        return None;
    }
    if ACTIONABLE_REVIEW_STATES
        .iter()
        .any(|phrase| has_affirmative_unresolved_phrase(&words, phrase))
    {
        return Some(WaitDisposition::Actionable);
    }
    (contains_any_phrase(&words, PENDING_STATES)
        || has_affirmative_resolution(&words)
        || contains_any_phrase(&words, &["no actionable feedback", "no review feedback"]))
    .then_some(WaitDisposition::Nonterminal)
}

fn has_affirmative_unresolved_phrase(words: &[&str], phrase: &str) -> bool {
    phrase_positions(words, phrase).any(|start| {
        let end = start + word_count(phrase);
        let before = &words[start.saturating_sub(3)..start];
        let after = &words[end..words.len().min(end + 5)];
        !before.iter().any(|word| NEGATIONS.contains(word))
            && (!(has_affirmative_resolution(before) || has_affirmative_resolution(after))
                || after.contains(&"unresolved"))
    })
}

fn has_affirmative_resolution(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        RESOLUTIONS.contains(word)
            && !words[index.saturating_sub(2)..index]
                .iter()
                .any(|prior| NEGATIONS.contains(prior))
    })
}

fn contains_any_phrase(words: &[&str], phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| phrase_positions(words, phrase).next().is_some())
}

fn has_nearby_words(words: &[&str], first: &str, second: &str, distance: usize) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        *word == first
            && words[index.saturating_sub(distance)..words.len().min(index + distance + 1)]
                .contains(&second)
    })
}

fn phrase_positions<'a>(
    tokens: &'a [&'a str],
    phrase: &'a str,
) -> impl Iterator<Item = usize> + 'a {
    let phrase = words(phrase);
    tokens
        .windows(phrase.len())
        .enumerate()
        .filter(move |(_, window)| *window == phrase)
        .map(|(index, _)| index)
}

fn word_count(text: &str) -> usize {
    words(text).len()
}

fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}
