pub(super) fn moved_token_coverage(removed: &str, extracted: &str) -> usize {
    let mut extracted_tokens = std::collections::HashMap::<&str, usize>::new();
    for token in extracted.split_whitespace() {
        *extracted_tokens.entry(token).or_default() += 1;
    }
    let mut total = 0usize;
    let mut moved = 0usize;
    for token in removed.split_whitespace() {
        total += 1;
        if let Some(count) = extracted_tokens.get_mut(token) {
            if *count > 0 {
                *count -= 1;
                moved += 1;
            }
        }
    }
    (total > 0)
        .then_some(moved.saturating_mul(4) / total)
        .unwrap_or(0)
}

pub(super) fn nonempty_line_count(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

pub(super) fn without_whitespace(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
