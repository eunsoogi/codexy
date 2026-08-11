const EVENT_SUBJECTS: &[(&str, SubjectKind)] = &[
    ("review feedback", SubjectKind::Review),
    ("review comment", SubjectKind::Review),
    ("requested changes", SubjectKind::ReviewState),
    ("changes requested", SubjectKind::ReviewState),
    ("maintainer feedback", SubjectKind::Review),
    ("feedback from maintainer", SubjectKind::Review),
    ("security review", SubjectKind::Review),
    ("connector review", SubjectKind::Review),
    ("parent authorization", SubjectKind::External),
    ("dependency integration", SubjectKind::External),
    ("resource slot", SubjectKind::External),
    ("async tool", SubjectKind::External),
    ("asynchronous tool", SubjectKind::External),
    ("implementation", SubjectKind::Operational),
    ("reviewer", SubjectKind::Review),
    ("sentinel", SubjectKind::External),
    ("resource", SubjectKind::External),
    ("operation", SubjectKind::External),
    ("review", SubjectKind::Review),
    ("tool", SubjectKind::External),
    ("work", SubjectKind::Operational),
    ("ci", SubjectKind::External),
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
const NEGATIONS: &[&str] = &["no", "not", "none", "without", "neither"];

const OPERATIONAL_PREDICATES: &[&str] = &["result", "wait"];
const REVIEW_PREDICATES: &[&str] = &["resolved", "unresolved", "open"];

#[derive(Clone, Copy, Eq, PartialEq)]
enum SubjectKind {
    External,
    Operational,
    Review,
    ReviewState,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SubjectGroup {
    External,
    Operational,
    Review,
}

#[derive(Clone, Copy)]
struct Subject {
    start: usize,
    end: usize,
    kind: SubjectKind,
}

pub(super) fn event_words(text: &str) -> Vec<Vec<&str>> {
    let words = words(text);
    let mut starts = vec![0];
    let subjects = subjects(&words);
    if let Some(first_subject) = subjects.first()
        && contains_any_phrase(&words[..first_subject.start], OPERATIONAL_PREDICATES)
    {
        starts.push(first_subject.start);
    }
    for (index, subject) in subjects.iter().enumerate().skip(1) {
        if starts_new_event(&subjects, index, &words) {
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
        || words.windows(2).enumerate().any(|(index, window)| {
            window == ["has", "returned"]
                && words[..index].iter().any(|word| NEGATIONS.contains(word))
        })
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
        if let Some((phrase, kind)) = EVENT_SUBJECTS
            .iter()
            .map(|(phrase, kind)| (words_of(phrase), *kind))
            .filter(|(phrase, _)| words[start..].starts_with(phrase))
            .max_by_key(|(phrase, _)| phrase.len())
        {
            let subject_start = start;
            start += phrase.len();
            subjects.push(Subject {
                start: subject_start,
                end: start,
                kind,
            });
        } else {
            start += 1;
        }
    }
    subjects
}

fn starts_new_event(subjects: &[Subject], index: usize, words: &[&str]) -> bool {
    let subject = subjects[index];
    let prior = subjects[index - 1];
    subject.start > 0
        && !is_coordinated_negation(words, subject.start)
        && (subject_group(subject.kind) != subject_group(prior.kind)
            || has_local_predicate(words, prior, subject.start))
}

fn subject_group(kind: SubjectKind) -> SubjectGroup {
    match kind {
        SubjectKind::External => SubjectGroup::External,
        SubjectKind::Operational => SubjectGroup::Operational,
        SubjectKind::Review | SubjectKind::ReviewState => SubjectGroup::Review,
    }
}

fn has_local_predicate(words: &[&str], subject: Subject, next_start: usize) -> bool {
    let words = &words[subject.end..next_start];
    contains_any_phrase(words, LIFECYCLE_STATES)
        || contains_any_phrase(words, REVIEW_PREDICATES)
        || contains_any_phrase(words, OPERATIONAL_PREDICATES)
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
