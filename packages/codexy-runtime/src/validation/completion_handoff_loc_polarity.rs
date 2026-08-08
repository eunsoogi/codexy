const STRUCTURAL_ACTIONS: &[&str] = &["moved", "extracted", "split", "separated", "removed"];

pub(super) fn is_negated(prefix: &str, suffix: &str, marker: &str) -> bool {
    prefix_negates_marker(prefix) || suffix_negates_marker(suffix, marker)
}

fn prefix_negates_marker(prefix: &str) -> bool {
    let words = words(
        prefix
            .rsplit([';', ':', ',', '\n'])
            .next()
            .unwrap_or(prefix),
    );
    let words = words
        .rsplit(|word| is_clause_boundary(word))
        .next()
        .unwrap_or(&[]);
    words.iter().any(|word| matches!(*word, "not" | "no")) || without_targets_marker(&words)
}

fn without_targets_marker(words: &[&str]) -> bool {
    matches!(words.last(), Some(&"without")) || matches!(words, [.., "without", "a" | "an" | "the"])
}

fn suffix_negates_marker(suffix: &str, marker: &str) -> bool {
    let words = words(suffix);
    let clauses = words
        .split(|word| is_clause_boundary(word))
        .collect::<Vec<_>>();
    clauses.iter().enumerate().any(|(index, clause)| {
        clause_negates_marker(clause, marker)
            && !(starts_with_did_not(clause) && follows_structural_action(&clauses, index))
    })
}

fn clause_negates_marker(clause: &[&str], marker: &str) -> bool {
    clause
        .windows(2)
        .any(|words| words[0] == "not" && !is_gerund(words[1]))
        || clause
            .windows(2)
            .any(|words| words[0] == "no" && is_marker_word(marker, words[1]))
}

fn starts_with_did_not(clause: &[&str]) -> bool {
    matches!(clause, ["did", "not", ..])
}

fn follows_structural_action(clauses: &[&[&str]], index: usize) -> bool {
    clauses[index + 1..].iter().any(|clause| {
        clause
            .first()
            .is_some_and(|word| STRUCTURAL_ACTIONS.contains(word))
    })
}

fn is_clause_boundary(word: &str) -> bool {
    matches!(word, "and" | "but" | "while")
}

fn is_gerund(word: &str) -> bool {
    word.ends_with("ing")
}

fn is_marker_word(marker: &str, word: &str) -> bool {
    marker.split_whitespace().any(|part| part == word)
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphabetic()))
        .filter(|word| !word.is_empty())
        .collect()
}
