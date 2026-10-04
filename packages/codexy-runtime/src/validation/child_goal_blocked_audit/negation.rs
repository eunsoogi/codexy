const EXPLICIT_NEGATIONS: &[&str] = &["no", "not", "none", "without", "neither"];

/// Recognizes whole-word negation forms used to distinguish prohibited claims from explicit denials.
pub(super) fn is_negation(word: &str) -> bool {
    EXPLICIT_NEGATIONS.contains(&word)
        || word
            .strip_suffix("n't")
            .or_else(|| word.strip_suffix("n’t"))
            .is_some_and(|stem| !stem.is_empty())
}

/// Keeps apostrophes inside tokens so contractions such as `don't` are not split into false positives.
pub(super) fn is_token_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '\'' | '’')
}
