use super::child_lane_active_thread_count::key_words;
use super::child_lane_active_thread_count_records::starts_count_clause;
use super::child_lane_active_thread_evidence::ThreadOwner;

pub(super) fn freed_capacity(line: &str) -> bool {
    let claim = freed_capacity_claim_text(line);
    let words = key_words(claim);
    let owner = ThreadOwner::from_line(claim);
    let Some(subject) = words.iter().position(|word| !word_in(word, "a|an|the")) else {
        return false;
    };
    let Some(completion) = words
        .iter()
        .position(|word| word_in(word, "archived|completed|finished|merged|removed|stopped"))
    else {
        return false;
    };
    subject <= completion
        && !words[subject..completion]
            .iter()
            .any(|word| word_in(word, "proof|review|test|tests|verification|evidence"))
        && is_capacity_subject(&words, subject, &owner)
        && "not|no|inactive".split('|').all(|marker| {
            !words[subject..=completion]
                .iter()
                .any(|word| word == marker)
        })
}

fn is_capacity_subject(words: &[String], subject: usize, owner: &ThreadOwner) -> bool {
    (!owner.issue_ids.is_empty() && word_in(&words[subject], "issue|pr|pull|request|merge"))
        || (owner.thread_id.is_some() && word_in(&words[subject], "thread|threads"))
        || (words[subject] == "child"
            && words
                .get(subject + 1)
                .is_some_and(|word| word == "thread" || word == "threads"))
}

fn word_in(word: &str, values: &str) -> bool {
    values.split('|').any(|value| word == value)
}

fn freed_capacity_claim_text(line: &str) -> &str {
    line.split_once(':')
        .filter(|(label, _)| starts_count_clause(label))
        .and_then(|(_, rest)| rest.split_once(',').map(|(_, trailing)| trailing))
        .unwrap_or(line)
}
