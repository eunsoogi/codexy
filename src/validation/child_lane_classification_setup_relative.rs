pub(super) fn relative_clause_owns_report_predicate(
    words: &[&str],
    start: usize,
    predicate: usize,
) -> bool {
    let Some(relative) = relative_marker(words, start, predicate) else {
        return false;
    };
    relative_finite_predicate(words, relative, predicate)
}

pub(super) fn main_clause_start(words: &[&str], start: usize, action: usize) -> Option<usize> {
    let relative = relative_marker(words, start, action)?;
    Some(
        (relative + 1..action)
            .find(|index| {
                report_clause_predicate(words[*index])
                    && !relative_finite_predicate(words, relative, *index)
            })
            .unwrap_or_else(|| predicate_chain_start(words, relative + 1, action)),
    )
}

pub(super) fn coordinates_relative_subject(
    words: &[&str],
    start: usize,
    conjunction: usize,
    action: usize,
) -> bool {
    let Some(relative) = relative_marker(words, start, action) else {
        return false;
    };
    relative < conjunction
        && !words[relative + 1..conjunction]
            .iter()
            .enumerate()
            .any(|(offset, _)| relative_finite_predicate(words, relative, relative + 1 + offset))
        && words[conjunction + 1..action]
            .iter()
            .enumerate()
            .any(|(offset, _)| relative_finite_predicate(words, relative, conjunction + 1 + offset))
}

pub(super) fn preserves_relative_subject_coordination(clause: &str) -> bool {
    let words = clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.iter().enumerate().any(|(index, word)| {
        *word == "and" && coordinates_relative_subject(&words, 0, index, words.len())
    })
}

pub(super) fn report_clause_predicate(word: &str) -> bool {
    matches!(
        word,
        "reports"
            | "reported"
            | "says"
            | "said"
            | "states"
            | "stated"
            | "explains"
            | "explained"
            | "notes"
            | "noted"
            | "tells"
            | "told"
    )
}

fn relative_marker(words: &[&str], start: usize, end: usize) -> Option<usize> {
    (start..end)
        .rev()
        .find(|index| matches!(words[*index], "who" | "whose" | "which"))
}

fn relative_finite_predicate(words: &[&str], relative: usize, predicate: usize) -> bool {
    relative_predicate_word(words[predicate])
        && relative_subject_head(words, relative, predicate).is_some_and(|head| {
            head < predicate
                && (head + 1..predicate).all(|index| !relative_predicate_word(words[index]))
        })
}

fn relative_subject_head(words: &[&str], relative: usize, predicate: usize) -> Option<usize> {
    if matches!(words[relative], "who" | "which") && relative + 1 == predicate {
        return Some(relative);
    }
    (relative + 1..predicate).find(|index| relative_subject_head_word(words[*index]))
}

fn relative_subject_head_word(word: &str) -> bool {
    matches!(word, "child" | "parent" | "orchestrator")
}

fn relative_predicate_word(word: &str) -> bool {
    report_clause_predicate(word) || matches!(word, "review" | "reviews" | "reviewed")
}

fn predicate_chain_start(words: &[&str], start: usize, action: usize) -> usize {
    let mut predicate = action;
    while predicate > start && is_predicate_chain_token(words[predicate - 1]) {
        predicate -= 1;
    }
    predicate
}

fn is_predicate_chain_token(word: &str) -> bool {
    word.ends_with("ly")
        || matches!(
            word,
            "not"
                | "never"
                | "do"
                | "does"
                | "did"
                | "is"
                | "are"
                | "was"
                | "were"
                | "be"
                | "been"
                | "being"
                | "has"
                | "have"
                | "had"
                | "will"
                | "would"
                | "can"
                | "could"
                | "may"
                | "might"
                | "should"
                | "must"
        )
}
