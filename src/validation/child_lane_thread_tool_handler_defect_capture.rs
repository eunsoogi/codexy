use super::child_lane_thread_tool_handler_issue_tracking::has_tracking_issue;
use super::child_lane_thread_tool_handler_issue_value::has_placeholder_or_pending_value;
use super::child_lane_thread_tool_handler_lane_mentions::has_unnegated_different_lane_phrase;
use super::child_lane_thread_tool_handler_no_route::has_false_no_route_answer;
use super::child_lane_thread_tool_handler_route_value::has_substantive_route_value;
pub(super) fn has_handler_marker_and_tool_name_in_defect_capture(
    evidence: &str,
    tool: &str,
) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        is_defect_capture_line(line)
            && !has_negated_fallback_route_field(line)
            && (has_handler_marker_and_tool_name_in_defect_clause(line, tool)
                && has_handler_handoff_fields(&defect_candidate_scope(&lines, index))
                || opens_defect_list(line)
                    && lines[index + 1..]
                        .iter()
                        .take_while(|following| is_list_item(following))
                        .enumerate()
                        .any(|following| {
                            let (offset, following) = following;
                            !has_negated_fallback_route_field(following)
                                && has_handler_marker(following)
                                && has_tool_name(following, tool)
                                && has_handler_handoff_fields(&list_item_candidate_scope(
                                    &lines,
                                    index,
                                    &lines[index + 1..],
                                    offset,
                                ))
                        }))
    })
}
pub(super) fn has_handler_marker_in_defect_capture(evidence: &str) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        is_defect_capture_line(line)
            && !has_negated_fallback_route_field(line)
            && (has_handler_marker_in_defect_clause(line)
                && has_handler_handoff_fields(&defect_candidate_scope(&lines, index))
                || opens_defect_list(line)
                    && lines[index + 1..]
                        .iter()
                        .take_while(|following| is_list_item(following))
                        .enumerate()
                        .any(|following| {
                            let (offset, following) = following;
                            !has_negated_fallback_route_field(following)
                                && has_handler_marker(following)
                                && has_handler_handoff_fields(&list_item_candidate_scope(
                                    &lines,
                                    index,
                                    &lines[index + 1..],
                                    offset,
                                ))
                        }))
    })
}
fn defect_candidate_scope(lines: &[&str], index: usize) -> String {
    let start = defect_scope_start(lines, index);
    let defect_lane = defect_lane_label(lines, start, index);
    let mut scoped = preceding_defect_scope_lines(lines, start, index, defect_lane.as_deref());
    scoped.push(current_defect_clause_scope_for_lane(
        lines[index],
        defect_lane.as_deref(),
    ));
    scoped.extend(
        lines[index + 1..]
            .iter()
            .take_while(|line| {
                is_unlisted_handoff_metadata_item_for_lane(line, defect_lane.as_deref())
                    || is_handoff_list_metadata_item_for_lane(line, defect_lane.as_deref())
                    || is_exact_handler_error_metadata_item(line)
            })
            .map(|line| {
                if is_handoff_list_metadata_item(line) {
                    strip_list_prefix(line)
                } else {
                    line
                }
            }),
    );
    scoped.join("\n")
}

fn preceding_defect_scope_lines<'a>(
    lines: &[&'a str],
    start: usize,
    index: usize,
    lane: Option<&str>,
) -> Vec<&'a str> {
    let mut skip_handoff_metadata_block = false;
    lines[start..index]
        .iter()
        .filter_map(|line| {
            if is_handoff_metadata_item_for_different_lane(line, lane) {
                skip_handoff_metadata_block = true;
                return None;
            }
            if skip_handoff_metadata_block && is_unlisted_or_list_handoff_metadata_item(line) {
                return None;
            }
            skip_handoff_metadata_block = false;
            Some(*line)
        })
        .collect()
}

fn is_unlisted_or_list_handoff_metadata_item(line: &str) -> bool {
    if is_handoff_list_metadata_item(line) {
        return true;
    }
    is_unlisted_handoff_metadata_item(line)
}

fn is_handoff_metadata_item_for_different_lane(line: &str, lane: Option<&str>) -> bool {
    let Some(lane) = lane else {
        return false;
    };
    let line = if is_handoff_list_metadata_item(line) {
        strip_list_prefix(line)
    } else {
        line
    };
    is_unlisted_handoff_metadata_item(line)
        && !is_unlisted_handoff_metadata_item_for_lane(line, Some(lane))
}

fn defect_header_candidate_scope(lines: &[&str], index: usize, lane: Option<&str>) -> String {
    let start = defect_scope_start(lines, index);
    let mut scoped = preceding_defect_scope_lines(lines, start, index, lane);
    scoped.push(current_defect_clause_scope_for_lane(lines[index], lane));
    scoped.join("\n")
}

fn defect_scope_start(lines: &[&str], index: usize) -> usize {
    let Some(previous_defect) = (0..index)
        .rev()
        .find(|candidate| is_defect_capture_line(lines[*candidate]))
    else {
        return 0;
    };
    let mut start = previous_defect + 1;
    while start < index && is_defect_trailing_metadata(lines[start]) {
        start += 1;
    }
    start
}

fn is_defect_trailing_metadata(line: &str) -> bool {
    is_unlisted_handoff_metadata_item(line)
        || is_handoff_list_metadata_item(line)
        || is_exact_handler_error_metadata_item(line)
}

fn list_item_candidate_scope(
    lines: &[&str],
    defect_index: usize,
    list_items: &[&str],
    index: usize,
) -> String {
    let start = defect_scope_start(lines, defect_index);
    let lane = defect_list_item_lane_label(list_items[index])
        .or_else(|| defect_lane_label(lines, start, defect_index));
    let header_scope = defect_header_candidate_scope(lines, defect_index, lane.as_deref());
    let mut scoped = vec![
        header_scope,
        strip_list_prefix(list_items[index]).to_string(),
    ];
    scoped.extend(
        list_items[index + 1..]
            .iter()
            .take_while(|line| is_handoff_list_metadata_item_for_lane(line, lane.as_deref()))
            .map(|line| strip_list_prefix(line).to_string()),
    );
    if let Some(shared_metadata) = shared_handoff_list_metadata(list_items, index, lane.as_deref())
    {
        scoped.extend(
            shared_metadata
                .iter()
                .map(|line| strip_list_prefix(line).to_string()),
        );
    }
    let list_end = list_items
        .iter()
        .position(|line| !is_list_item(line))
        .unwrap_or(list_items.len());
    scoped.extend(
        list_items[list_end..]
            .iter()
            .take_while(|line| is_unlisted_handoff_metadata_item_for_lane(line, lane.as_deref()))
            .map(|line| line.to_string()),
    );
    scoped.join("\n")
}

fn defect_list_item_lane_label(line: &str) -> Option<String> {
    let line = strip_list_prefix(line);
    prefixed_lane_label(line).or_else(|| mentioned_lane_label(line))
}

fn shared_handoff_list_metadata<'a>(
    list_items: &'a [&str],
    index: usize,
    lane: Option<&str>,
) -> Option<&'a [&'a str]> {
    let list_end = list_items
        .iter()
        .position(|line| !is_list_item(line))
        .unwrap_or(list_items.len());
    let metadata_start = (index + 1..list_end).find(|candidate| {
        list_items[*candidate..list_end].iter().all(|line| {
            is_handoff_list_metadata_item_for_lane(line, lane)
                && !has_handler_marker(strip_list_prefix(line))
        })
    })?;
    (metadata_start > index + 1).then_some(&list_items[metadata_start..list_end])
}

fn is_handoff_list_metadata_item(line: &str) -> bool {
    is_handoff_list_metadata_item_for_lane(line, None)
}

fn is_handoff_list_metadata_item_for_lane(line: &str, lane: Option<&str>) -> bool {
    let line = strip_list_prefix(line);
    is_unlisted_handoff_metadata_item_for_lane(line, lane)
}

fn is_unlisted_handoff_metadata_item(line: &str) -> bool {
    is_unlisted_handoff_metadata_item_for_lane(line, None)
}

fn is_unlisted_handoff_metadata_item_for_lane(line: &str, lane: Option<&str>) -> bool {
    let line_lower = line.to_ascii_lowercase();
    let Some(field_line) = strip_lane_label_prefix_for_lane(&line_lower, lane) else {
        return false;
    };
    let Some(scope_line) = strip_lane_label_prefix_for_lane_preserving_case(line, lane) else {
        return false;
    };
    (is_fallback_metadata_field(field_line)
        || [
            "separate dogfood issue",
            "separate dogfooding issue",
            "separate tracking issue",
            "tracking issue",
            "tracked in issue",
            "tracked by issue",
            "follow-up issue",
        ]
        .into_iter()
        .any(|field| field_line.starts_with(field)))
        && !names_later_lane_handoff(scope_line, lane)
}

fn names_later_lane_handoff(line: &str, lane: Option<&str>) -> bool {
    has_unnegated_different_lane_phrase(line)
        || mentioned_lane(line, lane)
            .is_some_and(|mentioned| lane.is_some_and(|lane| !mentioned.eq_ignore_ascii_case(lane)))
}

fn is_exact_handler_error_metadata_item(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    let line = strip_lane_label_prefix(&line);
    line.starts_with("exact missing-handler error:")
        && line.contains("no handler registered for tool:")
}

fn strip_lane_label_prefix(line: &str) -> &str {
    strip_lane_label_prefix_for_lane(line, None).unwrap_or(line)
}

fn strip_lane_label_prefix_for_lane<'a>(line: &'a str, lane: Option<&str>) -> Option<&'a str> {
    let Some(rest) = line.trim_start().strip_prefix("lane ") else {
        return Some(line);
    };
    let label_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .unwrap_or(rest.len());
    let label = rest[..label_end].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    if label.is_empty() {
        return Some(line);
    }
    if lane.is_some_and(|lane| lane != label) {
        return None;
    }
    Some(
        rest[label_end..].trim_start_matches(|ch: char| {
            ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.'
        }),
    )
}

fn strip_lane_label_prefix_for_lane_preserving_case<'a>(
    line: &'a str,
    lane: Option<&str>,
) -> Option<&'a str> {
    let trimmed = line.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest_lower) = lower.strip_prefix("lane ") else {
        return Some(line);
    };
    let rest = &trimmed["lane ".len()..];
    let label_end = rest_lower
        .find(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .unwrap_or(rest_lower.len());
    let label = rest[..label_end].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    if label.is_empty() {
        return Some(line);
    }
    if lane.is_some_and(|lane| !lane.eq_ignore_ascii_case(label)) {
        return None;
    }
    Some(
        rest[label_end..].trim_start_matches(|ch: char| {
            ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.'
        }),
    )
}

fn prefixed_lane_label(line: &str) -> Option<String> {
    let line = line.trim_start().to_ascii_lowercase();
    let rest = line.strip_prefix("lane ")?;
    let label_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .unwrap_or(rest.len());
    let label = rest[..label_end].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    (!label.is_empty()).then(|| label.to_string())
}

fn defect_lane_label(lines: &[&str], start: usize, index: usize) -> Option<String> {
    prefixed_lane_label(lines[index])
        .or_else(|| mentioned_lane_label(lines[index]))
        .or_else(|| {
            lines[start..index]
                .iter()
                .rev()
                .find_map(|line| lane_header_label(line))
        })
}

fn lane_header_label(line: &str) -> Option<String> {
    let line = line.trim_start().to_ascii_lowercase();
    let rest = line.strip_prefix("lane ")?;
    let label = rest.trim_end_matches(':').trim();
    (!label.is_empty() && label.bytes().all(|byte| byte.is_ascii_alphanumeric()))
        .then(|| label.to_string())
}

fn mentioned_lane<'a>(line: &'a str, lane: Option<&str>) -> Option<&'a str> {
    let lower = line.to_ascii_lowercase();
    ["for lane ", "in lane "]
        .into_iter()
        .find_map(|marker| mentioned_lane_after(line, &lower, marker, lane))
}

fn mentioned_lane_after<'a>(
    line: &'a str,
    lower: &str,
    marker: &str,
    lane: Option<&str>,
) -> Option<&'a str> {
    let lane_start = lower.find(marker)? + marker.len();
    let label = line[lane_start..]
        .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .next()
        .unwrap_or_default()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    is_lane_label_token(label)
        .then_some(label)
        .or_else(|| explicit_lane_marker_label(line, lane_start, marker, label).then_some(label))
        .or_else(|| scoped_lowercase_lane_label(label, lane).then_some(label))
}

fn mentioned_lane_label(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    ["for lane ", "in lane "].into_iter().find_map(|marker| {
        let lane_start = lower.find(marker)? + marker.len();
        let label = line[lane_start..]
            .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
            .next()
            .unwrap_or_default()
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        (is_lane_label_token(label)
            || explicit_lane_marker_label(line, lane_start, marker, label)
            || lowercase_lane_label_token(label))
        .then(|| label.to_ascii_lowercase())
    })
}

fn is_lane_label_token(label: &str) -> bool {
    !label.is_empty()
        && (label.bytes().all(|byte| byte.is_ascii_digit())
            || label.len() == 1 && label.bytes().all(|byte| byte.is_ascii_alphabetic())
            || label
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_uppercase()))
}

fn explicit_lane_marker_label(line: &str, lane_start: usize, marker: &str, label: &str) -> bool {
    let marker_start = lane_start.saturating_sub(marker.len());
    !label.is_empty()
        && line[marker_start..lane_start].contains("Lane ")
        && label.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn scoped_lowercase_lane_label(label: &str, lane: Option<&str>) -> bool {
    lane.is_some_and(|lane| lane.len() > 1) && lowercase_lane_label_token(label)
}

fn lowercase_lane_label_token(label: &str) -> bool {
    !label.is_empty()
        && label
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && !matches!(
            label,
            "context" | "handoff" | "metadata" | "review" | "setup" | "thread" | "workflow"
        )
}

fn current_defect_clause_scope_for_lane<'a>(line: &'a str, lane: Option<&str>) -> &'a str {
    let mentioned_lane;
    let lane = if lane.is_some() {
        lane
    } else {
        mentioned_lane = mentioned_lane_label(line);
        mentioned_lane.as_deref()
    };
    trim_at_other_lane_handoff_clause(current_defect_clause_scope(line), lane)
}

fn current_defect_clause_scope(line: &str) -> &str {
    let Some(defect_start) = line.find("defect") else {
        return line;
    };
    let search_start = defect_start + "defect".len();
    let lower = line.to_ascii_lowercase();
    "dogfooding defect|tool-exposure defect|dogfooding/tool-exposure defect"
        .split('|')
        .filter_map(|marker| {
            let index = search_start + lower[search_start..].find(marker)?;
            let prefix = lower[..index].trim_end();
            let suffix = lower[index + marker.len()..].trim_start();
            (is_defect_label_boundary(prefix) && suffix.starts_with(':')).then_some(index)
        })
        .min()
        .map_or(line, |next| &line[..next])
}

fn trim_at_other_lane_handoff_clause<'a>(line: &'a str, lane: Option<&str>) -> &'a str {
    let mut search_start = 0;
    while let Some(offset) = line[search_start..].find(';') {
        let separator = search_start + offset;
        let clause = line[separator + 1..].trim_start();
        if is_unlisted_handoff_metadata_item(clause)
            && !is_unlisted_handoff_metadata_item_for_lane(clause, lane)
        {
            return line[..separator].trim_end();
        }
        search_start = separator + 1;
    }
    line
}

fn is_defect_label_boundary(prefix: &str) -> bool {
    matches!(
        prefix.chars().next_back(),
        Some('.' | ';' | ',' | '-' | '\u{2013}' | '\u{2014}')
    )
}

fn has_handler_handoff_fields(evidence: &str) -> bool {
    let normalized = evidence.to_ascii_lowercase();
    has_fallback_route_or_none(&normalized) && has_tracking_issue(&normalized)
}

fn is_fallback_metadata_field(line: &str) -> bool {
    [
        "fallback route used:",
        "fallback route:",
        "fallback-route:",
        "fallback path:",
        "fallback-path:",
        "no fallback route:",
        "no fallback-route:",
        "no fallback path:",
        "no fallback-path:",
    ]
    .into_iter()
    .any(|field| line.starts_with(field))
}

fn has_fallback_route_or_none(evidence: &str) -> bool {
    evidence
        .lines()
        .map(str::trim)
        .any(|clause| has_explicit_no_route(clause) || has_concrete_fallback_route(clause))
}

fn has_concrete_fallback_route(clause: &str) -> bool {
    !has_negated_fallback_route(clause)
        && extract_fallback_route_value(clause).is_some_and(has_substantive_route_value)
}

fn extract_fallback_route_value(clause: &str) -> Option<&str> {
    [
        "fallback route used:",
        "fallback-route:",
        "fallback route:",
        "fallback-path:",
        "fallback path:",
    ]
    .into_iter()
    .find_map(|marker| clause.split_once(marker).map(|(_, value)| value))
    .map(trim_at_next_metadata_field)
}

fn trim_at_next_metadata_field(value: &str) -> &str {
    const NEXT_FIELDS: &str = "; tracking issue:|; tracked in issue:|; tracked by issue:|; follow-up issue:|; separate dogfood issue:|; separate dogfooding issue:|; separate tracking issue:";
    NEXT_FIELDS
        .split('|')
        .filter_map(|marker| value.find(marker))
        .min()
        .map_or(value, |index| &value[..index])
}

fn has_explicit_no_route(clause: &str) -> bool {
    const NO_ROUTE_MARKERS: &str = "no fallback route was available|no fallback route available|no fallback path was available|no fallback path available|no alternate route was available|no alternate route available|without a fallback route available|without fallback route available|without a fallback path available|without fallback path available";
    NO_ROUTE_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker))
        && !has_negated_no_route_claim(clause)
        && !has_placeholder_or_pending_value(
            clause
                .split_once(" because ")
                .map_or(clause, |(statement, _)| statement),
        )
}

fn has_negated_no_route_claim(clause: &str) -> bool {
    const NEGATED_NO_ROUTE_CLAIMS: &str = "false that no fallback route|false that no fallback path|false that no alternate route|not true that no fallback route|not true that no fallback path|not true that no alternate route|not the case that no fallback route|not the case that no fallback path|not the case that no alternate route";
    NEGATED_NO_ROUTE_CLAIMS
        .split('|')
        .any(|marker| clause.contains(marker))
        || has_false_no_route_answer(clause)
}

fn has_negated_fallback_route(clause: &str) -> bool {
    const NEGATED_FALLBACK_MARKERS: &str = "no fallback route:|no fallback path:|not a fallback route:|not a fallback path:|not a fallback route used:|not a fallback path used:|no fallback route evidence|no fallback path evidence|without fallback route evidence|without a fallback route|without fallback path evidence|without a fallback path";
    NEGATED_FALLBACK_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker))
}

fn is_defect_capture_line(line: &str) -> bool {
    line.contains("dogfooding defect")
        || line.contains("tool-exposure defect")
        || line.contains("dogfooding/tool-exposure defect")
}

fn has_handler_marker_in_defect_clause(line: &str) -> bool {
    defect_capture_clause(line).is_some_and(has_handler_marker)
}

fn has_handler_marker_and_tool_name_in_defect_clause(line: &str, tool: &str) -> bool {
    defect_capture_clause(line)
        .is_some_and(|clause| has_handler_marker(clause) && has_tool_name(clause, tool))
}

fn opens_defect_list(line: &str) -> bool {
    defect_capture_clause(line).is_some_and(|clause| clause.trim_end().ends_with(':'))
}

pub(super) fn has_negated_fallback_route_field(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    if has_bare_no_fallback_field_without_availability(&normalized) {
        return true;
    }
    [
        "not a fallback route:",
        "not a fallback-route:",
        "not a fallback path:",
        "not a fallback-path:",
        "no fallback route used:",
        "no fallback-route used:",
        "no fallback path used:",
        "no fallback-path used:",
        "fallback route not used:",
        "fallback-route not used:",
        "fallback path not used:",
        "fallback-path not used:",
        "fallback route: not used",
        "fallback-route: not used",
        "fallback path: not used",
        "fallback-path: not used",
        "not a fallback route used:",
        "not a fallback-route used:",
        "not a fallback path used:",
        "not a fallback-path used:",
        "without fallback route evidence",
        "without fallback-route evidence",
        "without fallback path evidence",
        "without fallback-path evidence",
    ]
    .into_iter()
    .any(|marker| normalized.contains(marker))
}

fn has_bare_no_fallback_field_without_availability(line: &str) -> bool {
    [
        "no fallback route:",
        "no fallback-route:",
        "no fallback path:",
        "no fallback-path:",
    ]
    .into_iter()
    .flat_map(|marker| line.split(marker).skip(1))
    .any(|value| {
        let value = value.trim_start();
        ![
            "no fallback route was available",
            "no fallback-route was available",
            "no fallback route available",
            "no fallback-route available",
            "no fallback path was available",
            "no fallback-path was available",
            "no fallback path available",
            "no fallback-path available",
            "without a fallback route available",
            "without a fallback-route available",
            "without fallback route available",
            "without fallback-route available",
            "without a fallback path available",
            "without a fallback-path available",
            "without fallback path available",
            "without fallback-path available",
        ]
        .into_iter()
        .any(|allowed| value.starts_with(allowed))
    })
}

fn defect_capture_clause(line: &str) -> Option<&str> {
    let clause = &line[line.find("defect")?..];
    let end = [". ", "; "]
        .into_iter()
        .filter_map(|boundary| clause.find(boundary))
        .min()
        .unwrap_or(clause.len());
    Some(&clause[..end])
}

fn is_list_item(line: &str) -> bool {
    strip_list_prefix(line).len() < line.trim_start().len()
}

fn strip_list_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(stripped) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        return stripped;
    }
    let Some((marker, stripped)) = trimmed.split_once(['.', ')']) else {
        return trimmed;
    };
    if marker.chars().all(|character| character.is_ascii_digit()) && stripped.starts_with(' ') {
        return stripped.trim_start();
    }
    trimmed
}

fn has_tool_name(line: &str, tool: &str) -> bool {
    line.contains(tool) || line.contains(&format!("codex_app.{tool}"))
}

fn has_handler_marker(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    [
        "no handler registered",
        "handler-missing",
        "missing-handler",
        "missing handler",
    ]
    .into_iter()
    .any(|marker| normalized.contains(marker))
        && "captured|classified|recorded|reported|routed|tracked"
            .split('|')
            .any(|marker| normalized.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::has_handler_marker_and_tool_name_in_defect_capture;

    #[test]
    fn rejects_preceding_handoff_metadata_for_a_different_lane() {
        let evidence = r#"Fallback route: parent posted the handoff for Lane B.
Tracking issue: #246
Lane A dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread."#;

        assert!(
            !has_handler_marker_and_tool_name_in_defect_capture(evidence, "read_thread"),
            "Lane A defect capture must not borrow preceding Lane B fallback metadata"
        );
    }

    #[test]
    fn rejects_list_defect_trailing_handoff_metadata_for_a_different_lane() {
        let evidence = r#"Dogfooding/tool-exposure defect:
- Lane A: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: parent posted the handoff in the child thread for Lane B.
Tracking issue: #246"#;

        assert!(
            !has_handler_marker_and_tool_name_in_defect_capture(evidence, "read_thread"),
            "Lane A list defect capture must not borrow trailing Lane B fallback metadata"
        );
    }

    #[test]
    fn rejects_list_defect_bulleted_handoff_metadata_for_a_different_lane() {
        let evidence = r#"Dogfooding/tool-exposure defect:
- Lane A: recorded runtime missing-handler evidence for codex_app.read_thread.
- Fallback route: parent posted the handoff in the child thread for Lane B.
- Tracking issue: #246"#;

        assert!(
            !has_handler_marker_and_tool_name_in_defect_capture(evidence, "read_thread"),
            "Lane A list defect capture must not borrow bulleted Lane B fallback metadata"
        );
    }

    #[test]
    fn rejects_single_line_defect_bulleted_handoff_metadata_for_a_different_lane() {
        let evidence = r#"Lane A:
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
- Fallback route: parent posted the handoff in the child thread for Lane B.
- Tracking issue: #246"#;

        assert!(
            !has_handler_marker_and_tool_name_in_defect_capture(evidence, "read_thread"),
            "Lane A single-line defect capture must not borrow bulleted Lane B fallback metadata"
        );
    }

    #[test]
    fn rejects_list_defect_bulleted_handoff_metadata_for_a_multi_letter_lane() {
        let evidence = r#"lane alpha:
dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread.
- fallback route: no fallback route was available for lane beta.
- tracking issue: #246 in lane beta review thread."#;

        assert!(
            !has_handler_marker_and_tool_name_in_defect_capture(evidence, "read_thread"),
            "Lane alpha list defect capture must not borrow Lane beta fallback metadata"
        );
    }

    #[test]
    fn rejects_bulleted_preceding_handoff_metadata_for_another_lane() {
        let evidence = r#"Lane A:
- Fallback route: parent posted the handoff for another lane.
- Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread."#;

        assert!(
            !has_handler_marker_and_tool_name_in_defect_capture(evidence, "read_thread"),
            "Lane A defect capture must not borrow bulleted preceding metadata for another lane"
        );
    }
}
