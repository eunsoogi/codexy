pub(super) fn mentions_override(segment: &str) -> bool {
    segment
        .split(|character: char| !character.is_ascii_alphabetic())
        .any(|word| matches!(word, "override" | "overrides"))
}
