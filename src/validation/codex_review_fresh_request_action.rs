pub(super) fn has_codex_review_post_action(clause: &str) -> bool {
    ["post", "comment", "send"]
        .iter()
        .any(|word| has_word_with_codex_review_target(clause, word))
}

fn has_word_with_codex_review_target(clause: &str, word: &str) -> bool {
    let mut rest = clause;
    let mut offset = 0;
    while let Some(index) = rest.find(word) {
        let start = offset + index;
        let end = start + word.len();
        if super::codex_review_fresh_request::is_word_match(clause, start, end)
            && names_codex_review_target(clause[end..].trim_start())
        {
            return true;
        }
        offset = end;
        rest = &clause[offset..];
    }
    false
}

fn names_codex_review_target(target: &str) -> bool {
    let target = target
        .strip_prefix("a ")
        .or_else(|| target.strip_prefix("an "))
        .or_else(|| target.strip_prefix("the "))
        .unwrap_or(target);
    target.starts_with("@codex review") || target.starts_with("codex review")
}
