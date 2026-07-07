pub(super) fn documents_incomplete_or_blocked_state(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let trimmed = text.trim_start();
    trimmed.starts_with("blocked")
        || trimmed.starts_with("blocker")
        || trimmed.starts_with("waiting")
        || has_any(
            &text,
            &[
                "blocked:",
                "blocked on",
                "blocked by",
                "blocked due to",
                "now blocked",
                "goal blocked",
                "work is blocked",
                "blocker:",
                "this lane is not complete",
                "lane is not complete",
                "is not complete",
                "isn't complete",
                "isn't yet complete",
                "remains unresolved",
                "not ready for handoff",
                "not currently ready for handoff",
                "aren't ready for handoff",
                "aren't yet ready for handoff",
                "aren't currently ready for handoff",
                "isn't currently ready for handoff",
                "aren't applicable",
                "isn't applicable",
                "waiting:",
            ],
        )
}

pub(super) fn documents_preventive_adjacent_review(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let Some(start) = text.find("preventive adjacent review") else {
        return false;
    };
    let segment = &text[start..preventive_adjacent_review_end(&text, start)];
    let has_adjacent_subject = has_any(
        segment,
        &[
            "adjacent gap",
            "adjacent parser",
            "helper family",
            "parser variant",
            "workflow variant",
            "invariant",
            "sibling",
        ],
    );
    let has_focused_coverage = has_any(
        segment,
        &[
            "focused regression coverage",
            "preventive regression coverage",
            "regression coverage",
            "focused test",
            "focused tests",
        ],
    );
    let has_concrete_no_change_rationale =
        has_any(segment, &["no-change rationale", "no change rationale"])
            && segment.contains("inspected")
            && (segment.contains("function") || segment.contains("test"))
            && has_any(segment, &["invariants hold", "invariant holds"]);

    has_adjacent_subject && (has_focused_coverage || has_concrete_no_change_rationale)
}

fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn preventive_adjacent_review_end(text: &str, start: usize) -> usize {
    let suffix = &text[start..];
    [suffix.find('\n'), suffix.find(". ")]
        .into_iter()
        .flatten()
        .min()
        .map_or(text.len(), |index| start + index)
}
