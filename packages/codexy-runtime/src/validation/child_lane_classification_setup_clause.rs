use super::child_lane_classification_setup_condition::has_negative_condition_adjunct;
use super::child_lane_classification_setup_phrase::is_adjunct_preposition;

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
            || has_negative_condition_adjunct(&words[start..predicate_start])
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
        || is_clause_local_adjunct_token(words, index)
        || (words[index] == "and"
            && index > 0
            && index + 1 < words.len()
            && is_adverbial_modifier(words[index - 1])
            && is_adverbial_modifier(words[index + 1]))
}

fn is_clause_local_adjunct_token(words: &[&str], index: usize) -> bool {
    words[..=index]
        .iter()
        .rposition(|word| is_adjunct_preposition(word))
        .is_some_and(|preposition| {
            preposition > 0
                && is_auxiliary_chain_word(words[preposition - 1])
                && words[preposition..=index]
                    .iter()
                    .all(|word| !is_auxiliary_chain_word(word))
        })
}

fn is_modal(word: &str) -> bool {
    matches!(
        word,
        "can" | "may" | "might" | "will" | "would" | "could" | "should" | "must" | "shall"
    )
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
    ) || is_modal(word)
        || is_negated_finite_auxiliary(word)
        || is_adverbial_modifier(word)
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
        .position(is_clause_boundary)
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
        .enumerate()
        .any(|(offset, word)| {
            (*word != "not" || !introduces_contrastive_actor(words, start + offset))
                && (matches!(*word, "not" | "never" | "neither")
                    || is_negated_finite_auxiliary(word))
        })
        || has_negative_condition_adjunct(&words[start..action])
        || words[start..action]
            .iter()
            .enumerate()
            .any(|(offset, word)| {
                *word == "without"
                    && words[start + offset + 1..action]
                        .iter()
                        .all(|word| is_adverbial_modifier(word))
            })
        || words[start..action].windows(2).any(is_contracted_negator)
}

fn introduces_contrastive_actor(words: &[&str], negator: usize) -> bool {
    let actor = match words.get(negator + 1).copied() {
        Some("a" | "an" | "the") => words.get(negator + 2).copied(),
        actor => actor,
    };
    matches!(actor, Some("child" | "parent" | "orchestrator"))
}

fn is_contracted_negator(pair: &[&str]) -> bool {
    matches!(pair, ["can", "t"])
        || matches!(pair, [auxiliary, "t"] if is_negated_finite_auxiliary(auxiliary))
}

fn is_negated_finite_auxiliary(word: &str) -> bool {
    matches!(
        word,
        "aren"
            | "cannot"
            | "couldn"
            | "didn"
            | "hadn"
            | "hasn"
            | "haven"
            | "isn"
            | "mustn"
            | "shan"
            | "shouldn"
            | "wasn"
            | "weren"
            | "won"
            | "wouldn"
    )
}

fn has_negated_setup_object(words: &[&str], start: usize, action: usize, end: usize) -> bool {
    let object = action + usize::from(matches!(words.get(action + 1), Some(&"up" | &"out"))) + 1;
    let negates_object = |before: &[&str]| before.contains(&"no");
    words[object..end]
        .iter()
        .position(|word| matches!(*word, "branch" | "worktree"))
        .is_some_and(|offset| negates_object(&words[object..object + offset]))
        || words[start..action]
            .iter()
            .rposition(|word| matches!(*word, "branch" | "worktree"))
            .is_some_and(|object| negates_object(&words[start..start + object]))
}
