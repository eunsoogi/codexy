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
#[derive(Clone, Copy)]
struct ReviewSubject {
    start: usize,
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
    if contains_any_phrase(&words, OPERATIONAL_NONTERMINAL_STATES) {
        return Some(WaitDisposition::Nonterminal);
    }
    reviewer
}

pub(in crate::validation) fn classify_reviewer_text(text: &str) -> Option<WaitDisposition> {
    let dispositions = review_clauses(text)
        .filter_map(|clause| classify_reviewer_words(&words(clause)))
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
        .filter(|(_, word)| matches!(**word, "resolved" | "unresolved"))
        .filter_map(|(index, word)| classify_resolution(words, &subjects, index, word))
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
    (contains_any_phrase(&words, PENDING_STATES)
        || subjects.iter().all(|subject| subject.negated)
        || contains_any_phrase(&words, &["no actionable feedback", "no review feedback"]))
    .then_some(WaitDisposition::Nonterminal)
}

fn classify_resolution(
    words: &[&str],
    subjects: &[ReviewSubject],
    index: usize,
    state: &str,
) -> Option<WaitDisposition> {
    let subject = closest_subject(subjects, index)?;
    if subject.negated {
        return Some(WaitDisposition::Nonterminal);
    }
    let state_negated = words[subject.end..index]
        .iter()
        .any(|word| NEGATIONS.contains(word));
    match (state, state_negated) {
        ("unresolved", false) | ("resolved", true) => Some(WaitDisposition::Actionable),
        ("resolved", false) | ("unresolved", true) => Some(WaitDisposition::Nonterminal),
        _ => None,
    }
}

fn closest_subject(subjects: &[ReviewSubject], state_index: usize) -> Option<ReviewSubject> {
    subjects.iter().copied().min_by_key(|subject| {
        if subject.end <= state_index {
            state_index - subject.end
        } else {
            subject.start.saturating_sub(state_index)
        }
    })
}

fn review_subjects(words: &[&str]) -> Vec<ReviewSubject> {
    let mut subjects = REVIEW_SUBJECTS
        .iter()
        .flat_map(|phrase| {
            let length = word_count(phrase);
            phrase_positions(words, phrase).map(move |start| ReviewSubject {
                start,
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
                start: index,
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
    words[index.saturating_sub(3)..index]
        .iter()
        .any(|word| NEGATIONS.contains(word))
}

fn subject_is_negated(words: &[&str], index: usize) -> bool {
    is_negated_at(words, index) || words[..index].iter().any(|word| *word == "neither")
}

fn review_clauses(text: &str) -> impl Iterator<Item = &str> {
    text.split(['.', ';', ':', '\n'])
        .filter(|clause| !clause.is_empty())
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

fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}
