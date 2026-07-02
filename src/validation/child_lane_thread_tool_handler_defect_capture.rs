use super::child_lane_thread_tool_handler_issue_reference::has_issue_reference;
use super::child_lane_thread_tool_handler_route_value::has_substantive_route_value;

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
    evidence
        .lines()
        .map(str::trim)
        .any(|clause| has_explicit_no_route(clause) || has_concrete_fallback_route(clause))
}

fn has_tracking_issue(evidence: &str) -> bool {
    const AFFIRMATIVE_MARKERS: &str = "separate dogfood issue|separate dogfooding issue|separate tracking issue|tracking issue|tracked in issue|tracked by issue|follow-up issue";
    handoff_clauses(evidence).any(|clause| {
        AFFIRMATIVE_MARKERS
            .split('|')
            .any(|marker| clause.contains(marker))
            && has_issue_reference(clause)
            && !has_negated_tracking_issue(clause)
            && !has_placeholder_or_pending_value(clause)
    })
}

fn has_concrete_fallback_route(clause: &str) -> bool {
    !has_negated_fallback_route(clause)
        && !has_negated_fallback_route_field(clause)
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
    const NO_ROUTE_MARKERS: &str = "no fallback route was available|no fallback route available|no alternate route was available|no alternate route available";
    NO_ROUTE_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker))
        && !has_negated_no_route_claim(clause)
        && !has_negated_fallback_route(clause)
        && !has_placeholder_or_pending_value(clause)
}

fn has_negated_no_route_claim(clause: &str) -> bool {
    const NEGATED_NO_ROUTE_CLAIMS: &str = "false that no fallback route|false that no alternate route|not true that no fallback route|not true that no alternate route|not the case that no fallback route|not the case that no alternate route";
    NEGATED_NO_ROUTE_CLAIMS
        .split('|')
        .any(|marker| clause.contains(marker))
}

fn has_negated_fallback_route(clause: &str) -> bool {
    const NEGATED_FALLBACK_MARKERS: &str = "no fallback route evidence|no fallback path evidence|without fallback route evidence|without a fallback route|without fallback path evidence|without a fallback path";
    NEGATED_FALLBACK_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker))
}

fn has_negated_fallback_route_field(clause: &str) -> bool {
    ["no fallback route:", "no fallback path:"]
        .into_iter()
        .any(|marker| clause.contains(marker))
}

fn has_negated_tracking_issue(clause: &str) -> bool {
    const NEGATED_TRACKING_ISSUE_MARKERS: &str = "no separate dogfood issue|no separate dogfooding issue|no issue was created|no issue created|no issue has been created|no issue filed|no issue was filed|no issue has been filed|has not been created|hasn't been created|has not been filed|hasn't been filed|no separate tracking issue|no tracking issue|no follow-up issue|no separate follow-up issue|not filed|wasn't created|wasn't filed|not a follow-up issue|not a separate follow-up issue|without a separate dogfood issue|without a separate dogfooding issue|without a separate tracking issue|without tracking issue|without a follow-up issue|without follow-up issue";
    NEGATED_TRACKING_ISSUE_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker))
        || has_negated_issue_lifecycle(clause)
}

fn has_negated_issue_lifecycle(clause: &str) -> bool {
    let normalized = clause
        .replace("wasn't", "was not")
        .replace("hasn't", "has not")
        .replace("hadn't", "had not");
    ["was", "has", "had"].into_iter().any(|auxiliary| {
        ["created", "filed"].into_iter().any(|verb| {
            normalized.contains(&format!("issue {auxiliary} not been {verb}"))
                || normalized.contains(&format!("issue {auxiliary} not yet been {verb}"))
                || normalized.contains(&format!("issue {auxiliary} not {verb}"))
                || normalized.contains(&format!("issue {auxiliary} not yet {verb}"))
        })
    })
}

fn has_placeholder_or_pending_value(clause: &str) -> bool {
    const PENDING_PHRASES: &str =
        "not created|not available|not provided|not yet|will be|to be created";
    PENDING_PHRASES
        .split('|')
        .any(|marker| clause.contains(marker))
        || clause.split_once(':').is_some_and(|(_, value)| {
            "none|n/a|tbd|pending|missing|absent|unavailable"
                .split('|')
                .any(|placeholder| value.trim() == placeholder)
        })
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
        && "captured|classified|recorded|reported|routed|tracked"
            .split('|')
            .any(|marker| normalized.contains(marker))
}
