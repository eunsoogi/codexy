pub(super) fn documents_incomplete_or_blocked_state(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let trimmed = text.trim_start();
    trimmed.starts_with("blocked")
        || trimmed.starts_with("blocker")
        || trimmed.starts_with("waiting")
        || has_unresolved_thread_waiting_state(&text)
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
                "is not currently complete",
                "isn't complete",
                "isn't yet complete",
                "not currently complete",
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

fn has_unresolved_thread_waiting_state(text: &str) -> bool {
    has_unnegated(text, "remains unresolved")
        && has_any(
            text,
            &[
                "this lane is not complete",
                "lane is not complete",
                "is not complete",
                "isn't complete",
                "isn't yet complete",
                "not ready for handoff",
                "aren't ready for handoff",
                "aren't yet ready for handoff",
                "not currently ready for handoff",
                "aren't currently ready for handoff",
                "isn't currently ready for handoff",
                "not applicable",
                "isn't applicable",
                "aren't applicable",
                "waiting:",
                "blocked:",
                "blocker:",
            ],
        )
}

pub(super) fn documents_preventive_adjacent_review(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let Some(start) = text.find("preventive adjacent review") else {
        return false;
    };
    let segment = &text[start..preventive_adjacent_review_end(&text, start)];
    let has_adjacent_subject = has_unnegated_any(
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
    let has_focused_coverage = has_unnegated_any(
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
        has_unnegated_any(segment, &["no-change rationale", "no change rationale"])
            && segment.contains("inspected")
            && (segment.contains("function") || segment.contains("test"))
            && has_any(segment, &["invariants hold", "invariant holds"])
            && has_substantive_rationale(segment);

    has_adjacent_subject && (has_focused_coverage || has_concrete_no_change_rationale)
}

fn has_unnegated_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| has_unnegated(text, needle))
}

fn has_unnegated(text: &str, needle: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(needle) {
        let start = offset + index;
        let end = start + needle.len();
        if !is_negated_match(&text[..start]) && !is_post_negated_match(&text[end..]) {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn is_negated_match(prefix: &str) -> bool {
    let local_start = prefix
        .rfind(['\n', ',', ';', ':'])
        .map_or(0, |index| index + 1);
    let local = prefix[local_start..].trim_end();
    local.split_ascii_whitespace().any(|word| {
        matches!(
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()),
            "no" | "not" | "without" | "missing" | "lacks" | "lack" | "none"
        )
    })
}

fn is_post_negated_match(suffix: &str) -> bool {
    let local_end = suffix
        .find(['\n', ',', ';'])
        .or_else(|| suffix.find(". "))
        .unwrap_or(suffix.len());
    let local = suffix[..local_end].trim_start();
    [
        "is not",
        "isn't",
        "are not",
        "aren't",
        "was not",
        "wasn't",
        "were not",
        "weren't",
        "not needed",
        "missing",
        "does not exist",
        "doesn't exist",
    ]
    .iter()
    .any(|negation| local.starts_with(negation))
}

fn has_substantive_rationale(segment: &str) -> bool {
    let Some((_, rationale)) = segment.rsplit_once("because") else {
        return false;
    };
    let rationale = rationale
        .trim_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '.' | ',' | ';' | ':'));
    !rationale.is_empty()
        && !has_any(
            rationale,
            &[
                "not applicable",
                "not-applicable",
                "n/a",
                "none",
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

fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn preventive_adjacent_review_end(text: &str, start: usize) -> usize {
    let suffix = &text[start..];
    [
        suffix.find("\n\n"),
        suffix.find("\n#"),
        suffix.find("\nreview "),
    ]
    .into_iter()
    .flatten()
    .min()
    .map_or(text.len(), |index| start + index)
}
