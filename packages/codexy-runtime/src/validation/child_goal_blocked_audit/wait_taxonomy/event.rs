const EVENT_BOUNDARIES: &[&str] = &[
    "but", "however", "yet", "although", "though", "whereas", "while", "when", "unless",
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

pub(super) fn event_words(text: &str) -> Vec<Vec<&str>> {
    let words = words(text);
    let mut events = Vec::new();
    let mut start = 0;
    for (index, word) in words.iter().enumerate() {
        if EVENT_BOUNDARIES.contains(word) {
            push_event(&mut events, &words[start..index]);
            start = index + 1;
        }
    }
    push_event(&mut events, &words[start..]);
    events
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

fn push_event<'a>(events: &mut Vec<Vec<&'a str>>, words: &[&'a str]) {
    if !words.is_empty() {
        events.push(words.to_vec());
    }
}

fn contains_any_phrase(words: &[&str], phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| phrase_positions(words, phrase))
}

fn phrase_positions(words: &[&str], phrase: &str) -> bool {
    let phrase = words_of(phrase);
    words.windows(phrase.len()).any(|window| window == phrase)
}

fn words_of(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}
