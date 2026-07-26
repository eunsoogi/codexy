pub(super) fn has_negative_condition_adjunct(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(under, word)| {
        *word == "under"
            && words[under + 1..]
                .iter()
                .position(|word| *word == "circumstances")
                .is_some_and(|circumstances| {
                    is_negative_condition_phrase(&words[under + 1..under + 1 + circumstances])
                })
    })
}

fn is_negative_condition_phrase(phrase: &[&str]) -> bool {
    phrase
        .iter()
        .position(|word| *word == "no")
        .is_some_and(|no| {
            phrase[..no]
                .iter()
                .chain(&phrase[no + 1..])
                .all(|word| !is_condition_phrase_boundary(word))
        })
}

fn is_condition_phrase_boundary(word: &&str) -> bool {
    matches!(
        *word,
        "a" | "an"
            | "the"
            | "and"
            | "or"
            | "but"
            | "at"
            | "by"
            | "for"
            | "from"
            | "given"
            | "in"
            | "on"
            | "to"
            | "under"
            | "with"
            | "without"
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
    )
}
