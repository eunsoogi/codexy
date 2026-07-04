pub(super) fn has_handler_marker_and_tool_name_in_defect_capture(
    evidence: &str,
    tool: &str,
) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        is_defect_capture_line(line)
            && !has_negated_fallback_route_field(line)
            && (has_handler_marker_and_tool_name_in_defect_clause(line, tool)
                || opens_defect_list(line)
                    && lines[index + 1..]
                        .iter()
                        .take_while(|following| is_list_item(following))
                        .any(|following| {
                            !has_negated_fallback_route_field(following)
                                && has_handler_marker(following)
                                && has_tool_name(following, tool)
                        }))
    })
}

pub(super) fn has_handler_marker_in_defect_capture(evidence: &str) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        is_defect_capture_line(line)
            && !has_negated_fallback_route_field(line)
            && (has_handler_marker_in_defect_clause(line)
                || opens_defect_list(line)
                    && lines[index + 1..]
                        .iter()
                        .take_while(|following| is_list_item(following))
                        .any(|following| {
                            !has_negated_fallback_route_field(following)
                                && has_handler_marker(following)
                        }))
    })
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
