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
    let mut words = words(suffix).into_iter().peekable();
    while let Some(word) = words.next() {
        if is_clause_boundary(word) {
            return false;
        }
        if word == "not" && !words.peek().is_some_and(|word| is_gerund(word)) {
            return true;
        }
        if word == "no"
            && words
                .next()
                .is_some_and(|word| is_marker_word(marker, word))
        {
            return true;
        }
    }
    false
}

fn is_clause_boundary(word: &str) -> bool {
    matches!(word, "and" | "but" | "while")
}

fn is_gerund(word: &&str) -> bool {
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
