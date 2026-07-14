use super::{
    has_false_no_route_answer, has_placeholder_or_pending_value, has_substantive_route_value,
    has_tracking_issue,
};

pub(crate) fn has_handler_handoff_fields(evidence: &str) -> bool {
    let normalized = evidence.to_ascii_lowercase();
    has_fallback_route_or_none(&normalized) && has_tracking_issue(&normalized)
}

pub(crate) fn is_fallback_metadata_field(line: &str) -> bool {
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

pub(crate) fn has_fallback_route_or_none(evidence: &str) -> bool {
    evidence
        .lines()
        .map(str::trim)
        .any(|clause| has_explicit_no_route(clause) || has_concrete_fallback_route(clause))
}

pub(crate) fn has_concrete_fallback_route(clause: &str) -> bool {
    !has_negated_fallback_route(clause)
        && extract_fallback_route_value(clause).is_some_and(has_substantive_route_value)
}

pub(crate) fn extract_fallback_route_value(clause: &str) -> Option<&str> {
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

pub(crate) fn trim_at_next_metadata_field(value: &str) -> &str {
    const NEXT_FIELDS: &str = "; tracking issue:|; tracked in issue:|; tracked by issue:|; follow-up issue:|; separate dogfood issue:|; separate dogfooding issue:|; separate tracking issue:";
    NEXT_FIELDS
        .split('|')
        .filter_map(|marker| value.find(marker))
        .min()
        .map_or(value, |index| &value[..index])
}

pub(crate) fn has_explicit_no_route(clause: &str) -> bool {
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

pub(crate) fn has_negated_no_route_claim(clause: &str) -> bool {
    const NEGATED_NO_ROUTE_CLAIMS: &str = "false that no fallback route|false that no fallback path|false that no alternate route|not true that no fallback route|not true that no fallback path|not true that no alternate route|not the case that no fallback route|not the case that no fallback path|not the case that no alternate route";
    NEGATED_NO_ROUTE_CLAIMS
        .split('|')
        .any(|marker| clause.contains(marker))
        || has_false_no_route_answer(clause)
}

pub(crate) fn has_negated_fallback_route(clause: &str) -> bool {
    const NEGATED_FALLBACK_MARKERS: &str = "no fallback route:|no fallback path:|not a fallback route:|not a fallback path:|not a fallback route used:|not a fallback path used:|no fallback route evidence|no fallback path evidence|without fallback route evidence|without a fallback route|without fallback path evidence|without a fallback path";
    NEGATED_FALLBACK_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker))
}
