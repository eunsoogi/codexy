pub(super) fn has_negative_label_value(suffix: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    [
        "not ready",
        "not yet ready",
        "not currently ready",
        "isn't ready",
        "isn't yet ready",
        "isn't currently ready",
        "aren't ready",
        "aren't yet ready",
        "aren't currently ready",
        "false",
        "not requested",
        "isn't requested",
        "aren't requested",
        "not applicable",
        "isn't applicable",
        "aren't applicable",
    ]
    .iter()
    .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
        || value
            .strip_prefix("no")
            .is_some_and(starts_with_standalone_label_boundary)
}

fn label_value(suffix: &str) -> Option<&str> {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let value = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('?'))?;
    Some(value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']))
}

fn starts_with_boundary(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn starts_with_standalone_label_boundary(rest: &str) -> bool {
    rest.is_empty()
        || rest
            .chars()
            .next()
            .is_some_and(|character| matches!(character, '.' | ';' | ',' | '\n' | '\r'))
}
