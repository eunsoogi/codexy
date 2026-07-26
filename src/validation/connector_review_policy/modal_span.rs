pub(super) fn clause_boundary(words: &[&str], raw_words: &[&str]) -> Option<usize> {
    words.iter().position(|word| *word == "comma").or_else(|| {
        words.iter().enumerate().find_map(|(index, word)| {
            (matches!(*word, "and" | "or" | "but" | "then")
                && raw_words
                    .get(index + 1)
                    .is_some_and(|next| next.chars().next().is_some_and(char::is_uppercase)))
            .then_some(index)
        })
    })
}
