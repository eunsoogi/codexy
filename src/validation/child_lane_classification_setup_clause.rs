pub(super) const SENTENCE_BOUNDARY: &str = "__codexy_sentence_boundary__";

pub(super) struct SetupClauseAnalysis {
    pub(super) start: usize,
    pub(super) prospective: bool,
    pub(super) negated: bool,
}

pub(super) fn analyze_setup_clause(
    words: &[&str],
    start: usize,
    action: usize,
    end: usize,
) -> SetupClauseAnalysis {
    let start = clause_start(words, start, action);
    let end = clause_end(words, action, end);
    let predicate_start = predicate_start(words, start, action);
    SetupClauseAnalysis {
        start,
        prospective: words[predicate_start..action].iter().any(|word| {
            matches!(
                *word,
                "will" | "shall" | "may" | "might" | "can" | "could" | "would" | "should" | "must"
            )
        }),
        negated: has_clause_negator(words, predicate_start, action)
            || has_negated_setup_object(words, start, action, end),
    }
}

fn predicate_start(words: &[&str], start: usize, action: usize) -> usize {
    let Some(auxiliary) = words[start..action]
        .iter()
        .rposition(|word| ["is", "are", "was", "were", "been", "being"].contains(word))
        .map(|offset| start + offset)
    else {
        return start;
    };
    let mut predicate = auxiliary;
    while predicate > start && is_auxiliary_chain_token(words, predicate - 1) {
        predicate -= 1;
    }
    predicate
}

fn is_auxiliary_chain_token(words: &[&str], index: usize) -> bool {
    is_auxiliary_chain_word(words[index])
        || (words[index] == "and"
            && index > 0
            && index + 1 < words.len()
            && is_adverbial_modifier(words[index - 1])
            && is_adverbial_modifier(words[index + 1]))
}

fn is_auxiliary_chain_word(word: &str) -> bool {
    matches!(
        word,
        "not"
            | "never"
            | "n"
            | "t"
            | "isn"
            | "aren"
            | "wasn"
            | "weren"
            | "hasn"
            | "haven"
            | "hadn"
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
            | "may"
            | "might"
            | "will"
            | "would"
            | "could"
            | "should"
            | "must"
            | "shall"
    ) || is_adverbial_modifier(word)
}

pub(super) fn is_adverbial_modifier(word: &str) -> bool {
    word.ends_with("ly")
        || matches!(
            word,
            "perhaps"
                | "maybe"
                | "still"
                | "already"
                | "yet"
                | "almost"
                | "quite"
                | "rather"
                | "just"
                | "even"
                | "also"
                | "ever"
                | "never"
                | "not"
        )
}

fn clause_start(words: &[&str], start: usize, action: usize) -> usize {
    words[start..action]
        .iter()
        .enumerate()
        .rposition(|(offset, word)| {
            is_clause_boundary(word)
                || (*word == "and" && !modifier_coordination(words, start + offset))
        })
        .map(|offset| start + offset + 1)
        .unwrap_or(start)
}

fn modifier_coordination(words: &[&str], conjunction: usize) -> bool {
    words
        .get(conjunction.wrapping_sub(1))
        .is_some_and(|word| is_adverbial_modifier(word))
}

fn clause_end(words: &[&str], action: usize, end: usize) -> usize {
    words[action + 1..end]
        .iter()
        .position(|word| is_clause_boundary(word))
        .map(|offset| action + offset + 1)
        .unwrap_or(end)
}

fn is_clause_boundary(word: &&str) -> bool {
    matches!(
        *word,
        SENTENCE_BOUNDARY
            | "although"
            | "because"
            | "but"
            | "however"
            | "then"
            | "whereas"
            | "while"
            | "yet"
    )
}

fn has_clause_negator(words: &[&str], start: usize, action: usize) -> bool {
    words[start..action]
        .iter()
        .any(|word| matches!(*word, "no" | "not" | "never" | "without" | "neither"))
        || words[start..action].windows(2).any(is_contracted_negator)
}

fn is_contracted_negator(pair: &[&str]) -> bool {
    matches!(
        pair,
        ["didn", "t"]
            | ["isn", "t"]
            | ["aren", "t"]
            | ["wasn", "t"]
            | ["weren", "t"]
            | ["hasn", "t"]
            | ["haven", "t"]
            | ["hadn", "t"]
    )
}

fn has_negated_setup_object(words: &[&str], start: usize, action: usize, end: usize) -> bool {
    let object = action + usize::from(matches!(words.get(action + 1), Some(&"up" | &"out"))) + 1;
    let negates_object = |before: &[&str]| before.iter().any(|word| *word == "no");
    words[object..end]
        .iter()
        .position(|word| matches!(*word, "branch" | "worktree"))
        .is_some_and(|offset| negates_object(&words[object..object + offset]))
        || words[start..action]
            .iter()
            .rposition(|word| matches!(*word, "branch" | "worktree"))
            .is_some_and(|object| negates_object(&words[start..start + object]))
}
