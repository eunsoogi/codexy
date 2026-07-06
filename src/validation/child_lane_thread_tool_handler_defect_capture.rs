use super::child_lane_thread_tool_handler_issue_tracking::has_tracking_issue;
use super::child_lane_thread_tool_handler_issue_value::has_placeholder_or_pending_value;
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
                                    &lines[index + 1..],
                                    offset,
                                    &defect_header_candidate_scope(&lines, index),
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
                                    &lines[index + 1..],
                                    offset,
                                    &defect_header_candidate_scope(&lines, index),
                                ))
                        }))
    })
}
fn defect_candidate_scope(lines: &[&str], index: usize) -> String {
    let start = (0..index)
        .rev()
        .find(|candidate| is_defect_capture_line(lines[*candidate]))
        .map_or(0, |candidate| candidate + 1);
    let end = lines[index + 1..]
        .iter()
        .position(|line| is_defect_capture_line(line))
        .map_or(lines.len(), |offset| index + 1 + offset);
    let mut scoped = lines[start..end].to_vec();
    scoped[index - start] = current_defect_clause_scope(lines[index]);
    scoped.join("\n")
}

fn defect_header_candidate_scope(lines: &[&str], index: usize) -> String {
    let start = (0..index)
        .rev()
        .find(|candidate| is_defect_capture_line(lines[*candidate]))
        .map_or(0, |candidate| candidate + 1);
    let mut scoped = lines[start..=index].to_vec();
    scoped[index - start] = current_defect_clause_scope(lines[index]);
    scoped.join("\n")
}

fn list_item_candidate_scope(list_items: &[&str], index: usize, header_scope: &str) -> String {
    let mut scoped = vec![header_scope, strip_list_prefix(list_items[index])];
    scoped.extend(
        list_items[index + 1..]
            .iter()
            .take_while(|line| is_handoff_list_metadata_item(line))
            .map(|line| strip_list_prefix(line)),
    );
    scoped.join("\n")
}

fn is_handoff_list_metadata_item(line: &str) -> bool {
    let line = strip_list_prefix(line).to_ascii_lowercase();
    is_fallback_metadata_field(&line)
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
        .any(|field| line.starts_with(field))
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
            (matches!(prefix.as_bytes().last(), Some(b'.' | b';')) && suffix.starts_with(':'))
                .then_some(index)
        })
        .min()
        .map_or(line, |next| &line[..next])
}

fn has_handler_handoff_fields(evidence: &str) -> bool {
    let normalized = evidence.to_ascii_lowercase();
    has_fallback_route_or_none(&normalized) && has_tracking_issue(&normalized)
}

fn is_fallback_metadata_field(line: &str) -> bool {
    [
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
    ["fallback route used:", "fallback route:", "fallback path:"]
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
    const NEGATED_NO_ROUTE_CLAIMS: &str = "false that no fallback route|false that no alternate route|not true that no fallback route|not true that no alternate route|not the case that no fallback route|not the case that no alternate route";
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
