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
    handoff_clauses(evidence).any(|clause| {
        clause.contains("no fallback route")
            || clause.contains("no alternate route")
            || has_concrete_fallback_route(clause)
    })
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
            && !has_placeholder_or_pending_value(clause)
    })
}

fn has_concrete_fallback_route(clause: &str) -> bool {
    ["fallback route", "fallback path", "fallback routed"]
        .into_iter()
        .any(|marker| clause.contains(marker))
        && !has_placeholder_or_pending_value(clause)
        && clause
            .split_once(':')
            .is_none_or(|(_, value)| !value.trim().is_empty())
}

fn has_issue_reference(clause: &str) -> bool {
    clause
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '#')
        .any(|word| {
            let issue_number = word.strip_prefix('#').unwrap_or(word);
            !issue_number.is_empty()
                && issue_number
                    .chars()
                    .all(|character| character.is_ascii_digit())
        })
}

fn has_placeholder_or_pending_value(clause: &str) -> bool {
    [
        "none",
        "n/a",
        "tbd",
        "pending",
        "not created",
        "not available",
        "not yet",
        "missing",
        "absent",
        "unavailable",
        "will be",
        "to be created",
    ]
    .into_iter()
    .any(|marker| clause.contains(marker))
}

fn handoff_clauses(evidence: &str) -> impl Iterator<Item = &str> {
    evidence.split(['\n', '.', ';']).map(str::trim)
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
