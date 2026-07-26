pub(super) fn has_negative_condition_adjunct(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(under, word)| {
        *word == "under"
            && words[under + 1..]
                .iter()
                .position(|word| *word == "circumstances")
                .is_some_and(|circumstances| {
                    let phrase = &words[under + 1..under + 1 + circumstances];
                    phrase
                        .iter()
                        .position(|word| *word == "no")
                        .is_some_and(|no| {
                            phrase[..no].iter().all(|word| is_condition_modifier(word))
                                && phrase[no + 1..]
                                    .iter()
                                    .all(|word| is_condition_modifier(word))
                        })
                })
    })
}

fn is_condition_modifier(word: &str) -> bool {
    word.ends_with("ly") || word.ends_with("able") || word.ends_with("ible")
}
