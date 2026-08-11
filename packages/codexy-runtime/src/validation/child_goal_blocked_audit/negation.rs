const EXPLICIT_NEGATIONS: &[&str] = &["no", "not", "none", "without", "neither"];

pub(super) fn is_negation(word: &str) -> bool {
    EXPLICIT_NEGATIONS.contains(&word)
        || word
            .strip_suffix("n't")
            .or_else(|| word.strip_suffix("n’t"))
            .is_some_and(|stem| !stem.is_empty())
}

pub(super) fn is_token_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '\'' | '’')
}
