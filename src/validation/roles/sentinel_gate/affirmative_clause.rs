pub(super) fn has_weakened_marker_prefix(prefix: &str) -> bool {
    let words = prefix
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let Some(verb_index) = words
        .iter()
        .rposition(|word| matches!(*word, "reference" | "record"))
    else {
        return false;
    };
    if words[..verb_index].last() != Some(&"must") {
        return false;
    }
    let suffix = &words[verb_index + 1..];
    let Some(first) = suffix.first() else {
        return false;
    };
    matches!(*first, "optional" | "optionally" | "waived" | "waiver")
        || matches!(*first, "if" | "when" | "where" | "unless" | "provided")
        || matches!(
            suffix,
            ["only", "if" | "when" | "where" | "unless" | "provided", ..]
        )
}

pub(super) fn has_quoted_marker_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim_end();
    matches!(prefix.chars().next_back(), Some('"' | '\'' | '`'))
        || prefix.chars().filter(|character| *character == '"').count() % 2 == 1
}
