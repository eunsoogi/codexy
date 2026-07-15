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

pub(crate) fn lane_label_prefix(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("Lane ")
        .or_else(|| trimmed.strip_prefix("lane "))?;
    let label_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .unwrap_or(rest.len());
    let label = rest[..label_end].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    ((is_lane_label_token(label) || is_lowercase_lane_label_token(label))
        && !is_reserved_lane_prefix_label(label))
    .then_some((
        label,
        rest[label_end..].trim_start_matches(|ch: char| {
            ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.'
        }),
    ))
}

fn is_reserved_lane_prefix_label(label: &str) -> bool {
    matches!(
        label.to_ascii_lowercase().as_str(),
        "owner" | "owners" | "ownership" | "metadata" | "type"
    )
}

pub(crate) fn strip_leading_lane_prefix_for_lane<'a>(
    line: &'a str,
    lane: Option<&str>,
) -> Option<&'a str> {
    let Some((label, rest)) = lane_label_prefix(line) else {
        return Some(line);
    };
    lane.is_none_or(|lane| lane.eq_ignore_ascii_case(label))
        .then_some(rest)
}

pub(crate) fn is_defect_label_boundary(prefix: &str) -> bool {
    matches!(
        prefix.chars().next_back(),
        Some('.' | ';' | ',' | '-' | '\u{2013}' | '\u{2014}')
    )
}

pub(crate) fn mentions_different_lane(line: &str, current_lane: &str) -> bool {
    if lane_label_prefix(line).is_some_and(|(lane, _)| !lane.eq_ignore_ascii_case(current_lane)) {
        return true;
    }
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
    plural_lane_mentions_different_lane(line, current_lane)
}

fn plural_lane_mentions_different_lane(line: &str, current_lane: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "for lanes ",
        "in lanes ",
        "assigned to lanes ",
        "targeting lanes ",
    ]
    .into_iter()
    .any(|marker| {
        let mut search_start = 0;
        while let Some(offset) = lower[search_start..].find(marker) {
            let start = search_start + offset;
            if !is_negated_lane_marker(&lower, start)
                && plural_lane_list_mentions_different_lane(
                    &line[start + marker.len()..],
                    current_lane,
                )
            {
                return true;
            }
            search_start = start + marker.len();
        }
        false
    })
}

fn plural_lane_list_mentions_different_lane(expression: &str, current_lane: &str) -> bool {
    let lane_expression = expression
        .replace("and/or", " and ")
        .replace("and-or", " and ")
        .replace(',', " , ")
        .replace('/', " / ")
        .replace([':', '-', '.'], " ");
    let mut tokens = lane_expression.split_whitespace();
    let Some(first) = tokens.next() else {
        return false;
    };
    if !is_lane_label_token(first) && !is_lowercase_lane_label_token(first) {
        return false;
    }
    if !first.eq_ignore_ascii_case(current_lane) {
        return true;
    }
    while let Some(connector) = tokens.next() {
        if !matches!(
            connector.to_ascii_lowercase().as_str(),
            "and" | "or" | "," | "/"
        ) {
            return false;
        }
        let Some(label) = tokens.next() else {
            return false;
        };
        if !is_lane_label_token(label) && !is_lowercase_lane_label_token(label) {
            return false;
        }
        if !label.eq_ignore_ascii_case(current_lane) {
            return true;
        }
    }
    false
}
