const EVENT_SUBJECTS: &[&str] = &[
    "review feedback",
    "review comment",
    "requested changes",
    "changes requested",
    "maintainer feedback",
    "feedback from maintainer",
    "security review",
    "connector review",
    "parent authorization",
    "dependency integration",
    "resource slot",
    "async tool",
    "asynchronous tool",
    "implementation",
    "reviewer",
    "sentinel",
    "resource",
    "operation",
    "review",
    "tool",
    "work",
    "ci",
];

const ASYNC_SUBJECTS: &[&str] = &[
    "ci",
    "sentinel",
    "review",
    "reviewer",
    "review feedback",
    "review comment",
    "security review",
    "connector review",
    "parent authorization",
    "dependency integration",
    "resource",
    "resource slot",
    "tool",
    "async tool",
    "asynchronous tool",
    "operation",
];

const LIFECYCLE_STATES: &[&str] = &[
    "pending",
    "waiting",
    "awaiting",
    "in progress",
    "processing",
    "queued",
    "running",
    "idle",
    "unavailable",
    "not returned",
    "not yet returned",
    "has not returned",
];

const REVIEW_STATES: &[&str] = &["resolved", "unresolved", "open"];
const OPERATIONAL_PREDICATES: &[&str] = &["result", "wait"];

#[derive(Clone, Copy)]
struct Subject {
    start: usize,
    end: usize,
}

pub(super) fn event_words(text: &str) -> Vec<Vec<&str>> {
    let words = words(text);
    let mut starts = vec![0];
    let complete_subjects = subjects(&words)
        .into_iter()
        .filter(|subject| subject_has_predicate(&words, *subject))
        .collect::<Vec<_>>();
    if let Some(first_subject) = complete_subjects.first()
        && contains_any_phrase(&words[..first_subject.start], OPERATIONAL_PREDICATES)
    {
        starts.push(first_subject.start);
    }
    for subject in complete_subjects.into_iter().skip(1) {
        if subject.start > 0 && !is_coordinated_negation(&words, subject.start) {
            starts.push(subject.start);
        }
    }
    starts.sort_unstable();
    starts.dedup();
    starts
        .iter()
        .enumerate()
        .filter_map(|(index, start)| {
            let end = starts.get(index + 1).copied().unwrap_or(words.len());
            (!words[*start..end].is_empty()).then(|| words[*start..end].to_vec())
        })
        .collect()
}

pub(super) fn has_lifecycle_state(words: &[&str]) -> bool {
    contains_any_phrase(words, LIFECYCLE_STATES)
}

pub(super) fn is_nonterminal_wait(words: &[&str]) -> bool {
    contains_any_phrase(words, ASYNC_SUBJECTS) && has_lifecycle_state(words)
}

pub(super) fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn subjects(words: &[&str]) -> Vec<Subject> {
    let mut subjects = Vec::new();
    let mut start = 0;
    while start < words.len() {
        if let Some(phrase) = EVENT_SUBJECTS
            .iter()
            .map(|phrase| words_of(phrase))
            .filter(|phrase| words[start..].starts_with(phrase))
            .max_by_key(Vec::len)
        {
            let end = start + phrase.len();
            subjects.push(Subject { start, end });
            start = end;
        } else {
            start += 1;
        }
    }
    subjects
}

fn subject_has_predicate(words: &[&str], subject: Subject) -> bool {
    let next_subject = subjects(words)
        .into_iter()
        .filter(|next| next.start > subject.start)
        .map(|next| next.start)
        .min()
        .unwrap_or(words.len());
    contains_any_phrase(&words[subject.end..next_subject], LIFECYCLE_STATES)
        || contains_any_phrase(&words[subject.end..next_subject], REVIEW_STATES)
        || contains_any_phrase(&words[subject.end..next_subject], OPERATIONAL_PREDICATES)
        || matches!(
            words[subject.start..subject.end],
            ["requested", "changes"] | ["changes", "requested"]
        )
}

fn is_coordinated_negation(words: &[&str], subject_start: usize) -> bool {
    words[..subject_start]
        .iter()
        .rposition(|word| *word == "neither")
        .is_some_and(|neither| words[neither + 1..subject_start].contains(&"nor"))
}

fn contains_any_phrase(words: &[&str], phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| {
        let phrase = words_of(phrase);
        words.windows(phrase.len()).any(|window| window == phrase)
    })
}

fn words_of(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}
