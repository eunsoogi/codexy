#[path = "wait_taxonomy/event.rs"]
mod event;

use super::negation::is_negation;
use event::{event_words, has_lifecycle_state, is_nonterminal_wait, words};

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

#[derive(Clone, Copy)]
struct ReviewSubject {
    end: usize,
    negated: bool,
}

pub(in crate::validation) fn classify_producer(value: &str) -> Option<WaitDisposition> {
    NONTERMINAL_PRODUCERS
        .contains(&value)
        .then_some(WaitDisposition::Nonterminal)
}

pub(in crate::validation) fn classify_wait_text(text: &str) -> Option<WaitDisposition> {
    let words = words(text);
    let reviewer = classify_reviewer_text(text);
    if reviewer == Some(WaitDisposition::Actionable) {
        return reviewer;
    }
    if event_words(text)
        .iter()
        .any(|event| is_nonterminal_wait(event))
        || contains_any_phrase(&words, OPERATIONAL_NONTERMINAL_STATES)
    {
        return Some(WaitDisposition::Nonterminal);
    }
    reviewer
}

pub(in crate::validation) fn classify_reviewer_text(text: &str) -> Option<WaitDisposition> {
    let dispositions = event_words(text)
        .iter()
        .filter_map(|event| classify_reviewer_words(event))
        .collect::<Vec<_>>();
    dispositions
        .contains(&WaitDisposition::Actionable)
        .then_some(WaitDisposition::Actionable)
        .or_else(|| {
            dispositions
                .contains(&WaitDisposition::Nonterminal)
                .then_some(WaitDisposition::Nonterminal)
        })
}

fn classify_reviewer_words(words: &[&str]) -> Option<WaitDisposition> {
    let subjects = review_subjects(words);
    if subjects.is_empty() {
        return None;
    }
    let resolution_states = words
        .iter()
        .enumerate()
        .filter(|(_, word)| matches!(**word, "resolved" | "unresolved" | "open"))
        .filter_map(|(index, word)| {
            let subject = subjects
                .iter()
                .filter(|subject| subject.end <= index)
                .max_by_key(|subject| subject.end)?;
            review_subject_owns_state(words, *subject, index)
                .then(|| classify_resolution(words, *subject, index, word))
                .flatten()
        })
        .collect::<Vec<_>>();
    if resolution_states.contains(&WaitDisposition::Actionable) {
        return Some(WaitDisposition::Actionable);
    }
    if resolution_states.contains(&WaitDisposition::Nonterminal) {
        return Some(WaitDisposition::Nonterminal);
    }
    if ACTIONABLE_REVIEW_STATES
        .iter()
        .any(|phrase| has_affirmative_phrase(words, phrase))
    {
        return Some(WaitDisposition::Actionable);
    }
    (has_lifecycle_state(words)
        || subjects.iter().all(|subject| subject.negated)
        || contains_any_phrase(&words, &["no actionable feedback", "no review feedback"]))
    .then_some(WaitDisposition::Nonterminal)
}

fn classify_resolution(
    words: &[&str],
    subject: ReviewSubject,
    index: usize,
    state: &str,
) -> Option<WaitDisposition> {
    if subject.negated {
        return Some(WaitDisposition::Nonterminal);
    }
    let state_negated = local_state_negation(words, index);
    match (state, state_negated) {
        ("unresolved" | "open", false) | ("resolved", true) => Some(WaitDisposition::Actionable),
        ("resolved", false) | ("unresolved" | "open", true) => Some(WaitDisposition::Nonterminal),
        _ => None,
    }
}

fn local_state_negation(words: &[&str], state_index: usize) -> bool {
    let predicate_start = words[..state_index]
        .iter()
        .rposition(|word| {
            matches!(
                *word,
                "resolved"
                    | "unresolved"
                    | "open"
                    | "pending"
                    | "waiting"
                    | "awaiting"
                    | "queued"
                    | "running"
                    | "idle"
                    | "unavailable"
                    | "processing"
            )
        })
        .map_or(0, |index| index + 1);
    words[predicate_start..state_index]
        .iter()
        .any(|word| is_negation(word))
}

fn review_subject_owns_state(words: &[&str], subject: ReviewSubject, state_index: usize) -> bool {
    state_index >= subject.end
        && !words[subject.end..state_index]
            .iter()
            .any(|word| matches!(*word, "resolved" | "unresolved" | "open"))
}

fn review_subjects(words: &[&str]) -> Vec<ReviewSubject> {
    let mut subjects = REVIEW_SUBJECTS
        .iter()
        .flat_map(|phrase| {
            let length = word_count(phrase);
            phrase_positions(words, phrase).map(move |start| ReviewSubject {
                end: start + length,
                negated: subject_is_negated(words, start),
            })
        })
        .collect::<Vec<_>>();
    for (index, word) in words.iter().enumerate() {
        if *word == "feedback"
            && words[index.saturating_sub(3)..words.len().min(index + 4)].contains(&"maintainer")
        {
            subjects.push(ReviewSubject {
                end: index + 1,
                negated: subject_is_negated(words, index),
            });
        }
    }
    subjects
}

fn has_affirmative_phrase(words: &[&str], phrase: &str) -> bool {
    phrase_positions(words, phrase).any(|start| !is_negated_at(words, start))
}

fn is_negated_at(words: &[&str], index: usize) -> bool {
    words[..index].iter().any(|word| is_negation(word))
}

fn subject_is_negated(words: &[&str], index: usize) -> bool {
    is_negated_at(words, index)
}

fn contains_any_phrase(words: &[&str], phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| phrase_positions(words, phrase).next().is_some())
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
