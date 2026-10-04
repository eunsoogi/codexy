pub(super) fn moved_line_coverage(removed: &str, extracted: &str) -> usize {
    // Score matched nonblank line occurrences in quarters (0 through 4); duplicate lines pair one-for-one, not by unique text.
    let mut extracted_lines = std::collections::HashMap::<&str, usize>::new();
    for line in extracted.lines().filter(|line| !line.trim().is_empty()) {
        *extracted_lines.entry(line).or_default() += 1;
    }
    let mut total = 0usize;
    let mut moved = 0usize;
    for line in removed.lines().filter(|line| !line.trim().is_empty()) {
        total += 1;
        if let Some(count) = extracted_lines.get_mut(line)
            && *count > 0
        {
            *count -= 1;
            moved += 1;
        }
    }
    moved
        .saturating_mul(4)
        .checked_div(total)
        .unwrap_or_default()
}

pub(super) fn nonempty_line_count(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

pub(super) fn without_whitespace(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
