pub(super) fn split_period_clauses(segment: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while let Some(relative) = segment[cursor..].find(". ") {
        let marker_start = cursor + relative;
        let next_start = marker_start + 2;
        if !is_numbered_thread_item(segment, marker_start, next_start) {
            clauses.push(&segment[start..marker_start]);
            start = next_start;
        }
        cursor = next_start;
    }
    clauses.push(&segment[start..]);
    clauses
}

fn is_numbered_thread_item(segment: &str, marker_start: usize, next_start: usize) -> bool {
    segment
        .as_bytes()
        .get(marker_start.saturating_sub(1))
        .is_some_and(u8::is_ascii_digit)
        && segment[next_start..]
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("thread")
}
