use super::child_lane_thread_tool_handler_issue_reference::has_issue_reference;

pub(super) fn has_handler_marker_and_tool_name_in_defect_capture(
    evidence: &str,
    tool: &str,
) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    has_handler_handoff_fields(evidence)
        && lines.iter().enumerate().any(|(index, line)| {
            is_defect_capture_line(line)
                && (has_handler_marker_and_tool_name_in_defect_clause(line, tool)
                    || opens_defect_list(line)
                        && lines[index + 1..]
                            .iter()
                            .take_while(|following| is_list_item(following))
                            .any(|following| {
                                has_handler_marker(following) && has_tool_name(following, tool)
                            }))
        })
}

pub(super) fn has_handler_marker_in_defect_capture(evidence: &str) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    has_handler_handoff_fields(evidence)
        && lines.iter().enumerate().any(|(index, line)| {
            is_defect_capture_line(line)
                && (has_handler_marker_in_defect_clause(line)
                    || opens_defect_list(line)
                        && lines[index + 1..]
                            .iter()
                            .take_while(|following| is_list_item(following))
                            .any(|following| has_handler_marker(following)))
        })
}

fn has_handler_handoff_fields(evidence: &str) -> bool {
    let normalized = evidence.to_ascii_lowercase();
    has_fallback_route_or_none(&normalized) && has_tracking_issue(&normalized)
}

fn has_fallback_route_or_none(evidence: &str) -> bool {
    handoff_clauses(evidence)
        .any(|clause| has_explicit_no_route(clause) || has_concrete_fallback_route(clause))
}

fn has_tracking_issue(evidence: &str) -> bool {
    let affirmative_markers = [
        "separate dogfood issue",
        "separate dogfooding issue",
        "separate tracking issue",
        "tracking issue",
        "tracked in issue",
        "tracked by issue",
        "follow-up issue",
    ];

    handoff_clauses(evidence).any(|clause| {
        affirmative_markers
            .into_iter()
            .any(|marker| clause.contains(marker))
            && has_issue_reference(clause)
            && !has_negated_tracking_issue(clause)
            && !has_placeholder_or_pending_value(clause)
    })
}

fn has_concrete_fallback_route(clause: &str) -> bool {
    ["fallback route", "fallback path", "fallback routed"]
        .into_iter()
        .any(|marker| clause.contains(marker))
        && !has_negated_fallback_route(clause)
        && !has_placeholder_or_pending_value(clause)
        && clause
            .split_once(':')
            .is_some_and(|(_, value)| has_substantive_route_value(value))
}

fn has_explicit_no_route(clause: &str) -> bool {
    [
        "no fallback route was available",
        "no fallback route available",
        "no alternate route was available",
        "no alternate route available",
    ]
    .into_iter()
    .any(|marker| clause.contains(marker))
        && !has_negated_fallback_route(clause)
        && !has_placeholder_or_pending_value(clause)
}

fn has_negated_fallback_route(clause: &str) -> bool {
    [
        "no fallback route evidence",
        "no fallback path evidence",
        "without fallback route evidence",
        "without a fallback route",
        "without fallback path evidence",
        "without a fallback path",
    ]
    .into_iter()
    .any(|marker| clause.contains(marker))
}

fn has_negated_tracking_issue(clause: &str) -> bool {
    [
        "no separate dogfood issue",
        "no separate dogfooding issue",
        "no separate tracking issue",
        "no tracking issue",
        "without a separate dogfood issue",
        "without a separate dogfooding issue",
        "without a separate tracking issue",
        "without tracking issue",
    ]
    .into_iter()
    .any(|marker| clause.contains(marker))
}

fn has_placeholder_or_pending_value(clause: &str) -> bool {
    let pending_phrases = [
        "not created",
        "not available",
        "not provided",
        "not yet",
        "will be",
        "to be created",
    ];

    pending_phrases
        .into_iter()
        .any(|marker| clause.contains(marker))
        || clause
            .split_once(':')
            .is_some_and(|(_, value)| has_placeholder_field_value(value))
}

fn has_placeholder_field_value(value: &str) -> bool {
    let trimmed = value.trim();
    [
        "none",
        "n/a",
        "tbd",
        "pending",
        "missing",
        "absent",
        "unavailable",
    ]
    .into_iter()
    .any(|placeholder| trimmed == placeholder)
}

fn has_substantive_route_value(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !["used", "routed", "available", "not used", "not routed"]
            .into_iter()
            .any(|weak_value| trimmed == weak_value)
        && trimmed
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        && (trimmed.split_whitespace().nth(1).is_some()
            || trimmed.contains("->")
            || trimmed.contains('/'))
}

fn handoff_clauses(evidence: &str) -> impl Iterator<Item = &str> {
    evidence
        .split(['\n', ';'])
        .flat_map(|clause| clause.split(". "))
        .map(str::trim)
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
    let trimmed = line.trim_start();
    trimmed.starts_with("- ") || trimmed.starts_with("* ")
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
        && [
            "captured",
            "classified",
            "recorded",
            "reported",
            "routed",
            "tracked",
        ]
        .into_iter()
        .any(|marker| normalized.contains(marker))
}
