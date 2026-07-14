pub(crate) fn is_negated_lane_marker(lower: &str, start: usize) -> bool {
    lower[..start]
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .next_back()
        .is_some_and(|token| matches!(token, "not" | "never" | "without"))
}

pub(crate) fn is_lane_label_token(label: &str) -> bool {
    !label.is_empty()
        && (label.bytes().all(|byte| byte.is_ascii_digit())
            || label.len() == 1 && label.bytes().all(|byte| byte.is_ascii_alphabetic())
            || label
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_uppercase()))
}

pub(crate) fn is_lowercase_lane_label_token(label: &str) -> bool {
    !label.is_empty()
        && label
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && !matches!(
            label,
            "context" | "handoff" | "metadata" | "review" | "setup" | "thread" | "workflow"
        )
}

pub(crate) fn is_defect_label_boundary(prefix: &str) -> bool {
    matches!(
        prefix.chars().next_back(),
        Some('.' | ';' | ',' | '-' | '\u{2013}' | '\u{2014}')
    )
}

pub(crate) fn mentions_different_lane(line: &str, current_lane: &str) -> bool {
    if super::super::child_lane_thread_tool_handler_lane_mentions::has_unnegated_different_lane_phrase(line) {
        return true;
    }
    let lower = line.to_ascii_lowercase();
    for marker in [
        "for lane ",
        "in lane ",
        "assigned to lane ",
        "targeting lane ",
    ] {
        let mut search_start = 0;
        while let Some(offset) = lower[search_start..].find(marker) {
            let start = search_start + offset;
            if !is_negated_lane_marker(&lower, start) {
                let label = line[start + marker.len()..]
                    .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
                    .next()
                    .unwrap_or_default()
                    .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
                if (is_lane_label_token(label) || is_lowercase_lane_label_token(label))
                    && !label.eq_ignore_ascii_case(current_lane)
                {
                    return true;
                }
            }
            search_start = start + marker.len();
        }
    }
    false
}
