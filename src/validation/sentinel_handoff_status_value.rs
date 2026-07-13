pub(super) fn is_missing_status_value(suffix: &str) -> bool {
    let clause = suffix
        .split(['.', '!', '?', ';', '\n'])
        .next()
        .unwrap_or_default();
    let words: Vec<_> = clause
        .trim_start_matches([' ', '\t', '-', '*'])
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.iter().enumerate().any(|(index, word)| {
        let mut value = words[index + 1..]
            .iter()
            .skip_while(|word| matches!(**word, "currently" | "still" | "now"));
        let subject = words[..index]
            .iter()
            .rev()
            .skip_while(|word| matches!(**word, "currently" | "still" | "now"))
            .next();
        matches!(*word, "is" | "was" | "were" | "are" | "be" | "been")
            && value
                .next()
                .is_some_and(|word| matches!(*word, "missing" | "absent" | "lacking"))
            && subject
                .is_none_or(|word| matches!(*word, "evidence" | "status" | "verdict" | "result"))
    })
}
