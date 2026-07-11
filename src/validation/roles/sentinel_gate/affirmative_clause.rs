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
        || matches!(
            suffix,
            [
                "if" | "when" | "where" | "unless" | "provided",
                "available" | "applicable" | "needed" | "possible" | "feasible" | "waived",
                ..
            ]
        )
        || matches!(
            suffix,
            [
                "only",
                "if" | "when" | "where" | "unless" | "provided",
                "available" | "applicable" | "needed" | "possible" | "feasible" | "waived",
                ..
            ]
        )
}
