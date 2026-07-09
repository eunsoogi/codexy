use super::preventive_adjacent_review_sections::{
    blocks_preventive_adjacent_segment, has_readiness_not_applicable_state,
    preventive_adjacent_review_end,
};
use super::preventive_adjacent_review_text::{
    has_any, has_current_blocker_phrase, has_false_blocked_or_waiting_value, has_pipe_any,
    has_unnegated, has_unnegated_any, has_unnegated_pipe, is_label_negated_match,
    is_stale_blocker_label_value,
};

pub(super) fn documents_incomplete_or_blocked_state(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let trimmed = text.trim_start();
    starts_with_true_blocker(trimmed)
        || starts_with_true_waiting(trimmed)
        || has_true_label_value(&text, "waiting:")
        || has_unresolved_thread_waiting_state(&text)
        || has_current_blocker_phrase(&text)
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
                && !is_stale_blocker_label_value(value)
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
    text.strip_prefix("waiting").is_some_and(|value| {
        !has_false_blocked_or_waiting_value(value) && !is_stale_blocker_label_value(value)
    })
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
    let mut documents_review = false;
    for (start, _) in text.match_indices("preventive adjacent review") {
        let segment = &text[start..preventive_adjacent_review_end(&text, start)];
        if blocks_preventive_adjacent_segment(&text, start, segment) {
            return false;
        }
        documents_review |= documents_preventive_adjacent_segment(segment);
    }
    documents_review
}
fn documents_preventive_adjacent_segment(segment: &str) -> bool {
    let has_adjacent_subject = has_unnegated_pipe(
        segment,
        "adjacent gap|adjacent parser|helper family|parser variant|workflow variant|sibling",
    );
    let has_code_surface = has_named_inspected_surface(
        segment,
        "function|functions|code surface|code surfaces|file|files",
        "test|tests|coverage|regression coverage|regression tests|focused tests",
    );
    let has_test_surface = has_named_inspected_surface(
        segment,
        "test|tests|coverage|regression coverage|regression tests|focused tests",
        "function|functions|code surface|code surfaces|file|files",
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
        .split_inclusive(['.', '\n', ';'])
        .any(|unit| {
            has_unnegated_pipe(
                unit,
                "regression coverage|regression tests|focused tests",
            ) && has_unnegated_pipe(
                unit,
                "adjacent gap|adjacent parser|helper family|parser variant|workflow variant|sibling",
            ) && has_executed_coverage_claim(unit)
                && !has_exact_comment_only_coverage(unit)
        })
}
fn has_executed_coverage_claim(unit: &str) -> bool {
    !has_requirement_template_context(unit)
        && has_unnegated_pipe(
            unit,
            "cover|covers|covered|exercise|exercises|exercised|run|ran|executed|passed|added|validate|validates|validated|check|checks|checked",
        )
}
fn has_requirement_template_context(unit: &str) -> bool {
    has_unnegated_pipe(
        unit,
        "can|could|may|might|must|should|would|requirement|checklist|template|needs to|need to",
    ) || (has_unnegated(unit, "required")
        && !has_unnegated_pipe(
            unit,
            "passed|ran|was run|were run|executed|was executed|were executed|added|was added|were added|validated|checked",
        ))
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
fn has_named_inspected_surface(segment: &str, surface_terms: &str, stop_terms: &str) -> bool {
    segment.split_inclusive(['.', '\n', ';']).any(|unit| {
        has_unnegated(unit, "inspected")
            && has_unnegated_pipe(unit, surface_terms)
            && has_specific_surface_name_after_surface(unit, surface_terms, stop_terms)
    })
}
fn has_specific_surface_name_after_surface(
    unit: &str,
    surface_terms: &str,
    stop_terms: &str,
) -> bool {
    surface_terms.split('|').any(|surface| {
        find_word(unit, surface).is_some_and(|surface_start| {
            let window_start = surface_start + surface.len();
            let window_end = stop_terms
                .split('|')
                .filter_map(|stop| find_word(&unit[window_start..], stop))
                .min()
                .map_or(unit.len(), |offset| window_start + offset);
            has_specific_surface_name(&unit[window_start..window_end])
        })
    })
}
fn find_word(text: &str, needle: &str) -> Option<usize> {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(needle) {
        let start = offset + index;
        let end = start + needle.len();
        let bounded = (start == 0 || !text.as_bytes()[start - 1].is_ascii_alphanumeric())
            && (end == text.len() || !text.as_bytes()[end].is_ascii_alphanumeric());
        if bounded {
            return Some(start);
        }
        offset = end;
        rest = &text[offset..];
    }
    None
}
fn has_specific_surface_name(text: &str) -> bool {
    text.split_ascii_whitespace().any(|word| {
        let clean = word.trim_matches(|ch: char| {
            !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '/' | '.'))
        });
        clean.contains('_')
            || clean.contains("::")
            || clean.ends_with(".rs")
            || (clean.contains('/') && (clean.contains('_') || clean.contains(".rs")))
    })
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
