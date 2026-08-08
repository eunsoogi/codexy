use super::sentinel_handoff::{SENTINEL_MARKERS, affirmed_phrase_starts, clause_bounds};

const REVIEWER_IDENTITY_NOISE_WORDS: &str = "pass|passed|passes|block|blocked|unobservable|returned|return|status|verdict|result|gate|reviewer|sentinel|codexy|packaged|current|head|exact|sha|oid|commit|on|for|the|this|a|an|as|planned|approved|approval|fallback|docs|separately|evidence|proof";
const STATUS_NOISE_WORDS: &str =
    "pass|passed|passes|block|blocked|test|tests|focused|but|before|after|waiting|wait|rerun|retry";

pub(super) fn pass_names_reviewer(text: &str, start: usize) -> bool {
    reviewer_name_before_status(text, start) || reviewer_name_after_status(text, start)
}

fn reviewer_name_before_status(text: &str, start: usize) -> bool {
    let context_start = last_status_context_boundary(&text[..start]).unwrap_or(0);
    let prefix = &text[context_start..start];
    let Some(marker_end) = last_sentinel_marker_end(prefix) else {
        return false;
    };
    has_reviewer_identity_words(&prefix[marker_end..])
}

fn reviewer_name_after_status(text: &str, start: usize) -> bool {
    let (_, clause_end) = clause_bounds(text, start);
    let evidence = &text[start..clause_end];
    let words: Vec<_> = evidence
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.windows(4).any(|window| {
        is_reviewer_identity_word(window[0])
            && window[1] == "reviewed"
            && matches!(window[2], "exact" | "current")
            && window[3] == "head"
    })
}

fn has_reviewer_identity_words(text: &str) -> bool {
    let words: Vec<_> = text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    !words.is_empty()
        && words.len() <= 4
        && words.iter().any(|word| is_reviewer_identity_word(word))
        && words
            .iter()
            .all(|word| !STATUS_NOISE_WORDS.split('|').any(|noise| *word == noise))
}

fn is_reviewer_identity_word(word: &str) -> bool {
    word.len() >= 2
        && word
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        && !REVIEWER_IDENTITY_NOISE_WORDS
            .split('|')
            .any(|noise| word == noise)
}

fn last_status_context_boundary(text: &str) -> Option<usize> {
    text.rfind(['.', '!', '?', ';', '\n'])
        .map(|index| index + 1)
}

fn last_sentinel_marker_end(text: &str) -> Option<usize> {
    SENTINEL_MARKERS
        .split('|')
        .filter_map(|phrase| {
            affirmed_phrase_starts(text, phrase)
                .last()
                .map(|start| start + phrase.len())
        })
        .max()
}
