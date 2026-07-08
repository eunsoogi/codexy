use super::preventive_adjacent_review_sections::{
    has_false_readiness_before_evidence, has_readiness_not_applicable_state,
    preventive_adjacent_review_end,
};
use super::preventive_adjacent_review_text::{
    has_any, has_false_blocked_or_waiting_value, has_pipe_any, has_unnegated, has_unnegated_any,
    has_unnegated_pipe, is_label_negated_match,
};

pub(super) fn documents_incomplete_or_blocked_state(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let trimmed = text.trim_start();
    starts_with_true_blocker(trimmed)
        || starts_with_true_waiting(trimmed)
        || has_true_label_value(&text, "waiting:")
        || has_unresolved_thread_waiting_state(&text)
        || has_unnegated_pipe(
            &text,
            "blocked on|blocked by|blocked due to|now blocked|goal blocked|work is blocked",
        )
        || has_true_blocked_or_blocker_label(&text)
}
fn starts_with_true_blocker(text: &str) -> bool {
    ["blockers", "blocked", "blocker"].iter().any(|prefix| {
        text.strip_prefix(prefix).is_some_and(|value| {
            value
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric())
                && !has_false_blocked_or_waiting_value(value)
        })
    })
}
fn has_true_blocked_or_blocker_label(text: &str) -> bool {
    ["blocked:", "blocker:", "blockers:"]
        .iter()
        .any(|label| has_true_label_value(text, label))
}
fn has_true_label_value(text: &str, label: &str) -> bool {
    text.match_indices(label).any(|(index, _)| {
        let has_boundary = index == 0 || !text.as_bytes()[index - 1].is_ascii_alphanumeric();
        let value = &text[index + label.len()..];
        has_boundary
            && !is_label_negated_match(&text[..index])
            && !has_false_blocked_or_waiting_value(value)
            && !is_stale_blocker_label_value(value)
    })
}
fn starts_with_true_waiting(text: &str) -> bool {
    text.strip_prefix("waiting")
        .is_some_and(|value| !has_false_blocked_or_waiting_value(value))
}
fn is_stale_blocker_label_value(value: &str) -> bool {
    let end = value.find('\n').unwrap_or(value.len());
    let value = value[..end].trim();
    has_any(value, &["previous", "previously", "historical", "earlier"])
        && has_any(value, &["resolved", "cleared"])
        && !has_any(
            value,
            &[
                "now blocked",
                "currently blocked",
                "pending",
                "still blocked",
                "still waiting",
            ],
        )
}
fn has_unresolved_thread_waiting_state(text: &str) -> bool {
    has_unnegated_any(text, &["remains unresolved", "remain unresolved"])
        && (has_pipe_any(
            text,
            "this lane is not complete|lane is not complete|is not complete|isn't complete|isn't yet complete|not currently complete|not ready for handoff|aren't ready for handoff|aren't yet ready for handoff|not currently ready for handoff|aren't currently ready for handoff|isn't currently ready for handoff",
        ) || has_readiness_not_applicable_state(text)
            || has_true_label_value(text, "waiting:"))
        || has_true_blocked_or_blocker_label(text)
}
pub(super) fn documents_preventive_adjacent_review(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let Some(start) = text.find("preventive adjacent review") else {
        return false;
    };
    if has_false_readiness_before_evidence(&text, start) {
        return false;
    }
    let segment = &text[start..preventive_adjacent_review_end(&text, start)];
    let has_adjacent_subject = has_unnegated_pipe(
        segment,
        "adjacent gap|adjacent parser|helper family|parser variant|workflow variant|sibling",
    );
    let has_code_surface =
        has_unnegated_pipe(segment, "function|functions|code surface|code surfaces");
    let has_test_surface = has_unnegated_pipe(
        segment,
        "test|tests|coverage|regression coverage|regression tests|focused tests",
    );
    let has_concrete_no_change_rationale = (has_adjacent_subject
        || has_any(segment, &["none of the sibling", "none of the adjacent"]))
        && has_unnegated_any(segment, &["no-change rationale", "no change rationale"])
        && has_unnegated(segment, "inspected")
        && has_code_surface
        && has_test_surface
        && has_unnegated_any(segment, &["invariants hold", "invariant holds"])
        && has_substantive_rationale(segment);
    has_focused_adjacent_coverage(segment) || has_concrete_no_change_rationale
}
fn has_focused_adjacent_coverage(segment: &str) -> bool {
    segment
        .split_inclusive(['.', '\n'])
        .any(|unit| {
            has_unnegated_pipe(
                unit,
                "regression coverage|regression tests|focused tests",
            ) && has_unnegated_pipe(
                unit,
                "adjacent gap|adjacent parser|helper family|parser variant|workflow variant|sibling",
            ) && !has_exact_comment_only_coverage(unit)
        })
}
fn has_exact_comment_only_coverage(unit: &str) -> bool {
    has_unnegated_any(
        unit,
        &[
            "only the exact comment",
            "only the exact review comment",
            "only the exact codex review comment",
            "only exact comment",
            "only exact review comment",
            "the exact comment only",
            "the exact review comment only",
            "the exact codex review comment only",
            "exact comment only",
            "exact review comment only",
            "exact-comment-only",
        ],
    )
}
fn has_substantive_rationale(segment: &str) -> bool {
    let Some((_, rationale)) = segment.rsplit_once("because") else {
        return false;
    };
    let rationale = rationale
        .trim_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '.' | ',' | ';' | ':'));
    !rationale.is_empty()
        && rationale != "none"
        && !has_any(
            rationale,
            &[
                "not applicable",
                "not-applicable",
                "n/a",
                "no change needed",
                "not needed",
                "does not apply",
                "doesn't apply",
                "out of scope",
                "irrelevant",
                "not relevant",
            ],
        )
}
