pub(super) fn has_missing_status_suffix(suffix: &str) -> bool {
    let evidence = suffix
        .trim_start_matches([' ', '\t'])
        .strip_prefix(':')
        .unwrap_or(suffix)
        .trim_start_matches([' ', '\t']);
    let words: Vec<_> = evidence
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let status_value = if words
        .first()
        .is_some_and(|word| matches!(*word, "status" | "evidence" | "proof"))
    {
        &words[1..]
    } else {
        &words
    };
    status_value
        .first()
        .is_some_and(|state| matches!(*state, "missing" | "absent" | "lacking"))
        || status_value
            .first()
            .zip(status_value.get(1))
            .is_some_and(|(verb, state)| {
                matches!(
                    (*verb, *state),
                    (
                        "is" | "was" | "were" | "are" | "be" | "been",
                        "missing" | "absent" | "lacking"
                    )
                )
            })
}
