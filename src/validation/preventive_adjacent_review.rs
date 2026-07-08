pub(super) fn documents_incomplete_or_blocked_state(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let trimmed = text.trim_start();
    starts_with_true_blocker(trimmed)
        || starts_with_true_waiting(trimmed)
        || has_unresolved_thread_waiting_state(&text)
        || has_unnegated_any(
            &text,
            &[
                "blocked on",
                "blocked by",
                "blocked due to",
                "now blocked",
                "goal blocked",
                "work is blocked",
            ],
        )
        || has_any(
            &text,
            &[
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
            ],
        )
        || has_true_blocked_or_blocker_label(&text)
}
fn starts_with_true_blocker(text: &str) -> bool {
    (text.starts_with("blocked") || text.starts_with("blocker")) && !has_false_heading_value(text)
}
fn has_false_heading_value(text: &str) -> bool {
    text.split_once(':')
        .is_some_and(|(_, value)| has_false_blocked_or_waiting_value(value))
}
fn has_true_blocked_or_blocker_label(text: &str) -> bool {
    ["blocked:", "blocker:", "blockers:"]
        .iter()
        .any(|label| has_true_label_value(text, label))
}
fn has_true_label_value(text: &str, label: &str) -> bool {
    text.match_indices(label).any(|(index, _)| {
        let has_boundary = index == 0 || !text.as_bytes()[index - 1].is_ascii_alphanumeric();
        has_boundary
            && !is_negated_match(&text[..index])
            && !has_false_blocked_or_waiting_value(&text[index + label.len()..])
    })
}
fn starts_with_true_waiting(text: &str) -> bool {
    text.starts_with("waiting") && !has_false_heading_value(text)
}
fn has_false_blocked_or_waiting_value(value: &str) -> bool {
    let value = value.trim_start();
    let first = value
        .split(|ch: char| !matches!(ch, '/' | '0'..='9' | 'a'..='z'))
        .next()
        .unwrap_or("");
    let rest = value[first.len()..].trim_start_matches([' ', '\t']);
    let terminal = rest.chars().next().is_none_or(|ch| ".;,\n\r".contains(ch));
    let false_modifier = ["active", "currently", "now", "remain"]
        .iter()
        .any(|modifier| rest.starts_with(modifier));
    let false_empty = matches!(first, "0" | "zero" | "none" | "no" | "false" | "n/a" | "na")
        && (terminal || false_modifier);
    false_empty
        || matches!(first, "resolved" | "cleared")
        || value.starts_with("not applicable")
        || value.starts_with("no blocker")
        || value.starts_with("no waiting")
        || value.starts_with("no current ")
}
fn has_unresolved_thread_waiting_state(text: &str) -> bool {
    has_unnegated(text, "remains unresolved")
        && (has_any(
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
            ],
        ) || has_readiness_not_applicable_state(text)
            || has_true_label_value(text, "waiting:"))
        || has_true_blocked_or_blocker_label(text)
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
            "sibling",
        ],
    );
    let has_focused_coverage = has_unnegated_any(
        segment,
        &["regression coverage", "regression tests", "focused test"],
    );
    let has_concrete_no_change_rationale = (has_adjacent_subject
        || has_any(segment, &["none of the sibling", "none of the adjacent"]))
        && has_unnegated_any(segment, &["no-change rationale", "no change rationale"])
        && has_unnegated(segment, "inspected")
        && (has_unnegated(segment, "function") || has_unnegated(segment, "test"))
        && has_unnegated_any(segment, &["invariants hold", "invariant holds"])
        && has_substantive_rationale(segment);
    (has_adjacent_subject && has_focused_coverage) || has_concrete_no_change_rationale
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
    let local = suffix[..local_end]
        .trim_start_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ':' | '-'));
    has_false_blocked_or_waiting_value(local)
        || has_any(local, &[" is missing", " not tested", " not covered"])
        || local.starts_with("s not ")
        || [
            "is not",
            "isn't",
            "are not",
            "aren't",
            "was not",
            "wasn't",
            "were not",
            "weren't",
            "is missing",
            "s are missing",
            "remains missing",
            "remain missing",
            "still missing",
            "not added",
            "not needed",
            "not run",
            "not executed",
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
fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}
fn has_readiness_not_applicable_state(text: &str) -> bool {
    ["pr readiness:", "readiness:"].iter().any(|label| {
        text.find(label).is_some_and(|index| {
            ["not applicable", "isn't applicable", "aren't applicable"]
                .iter()
                .any(|state| text[index + label.len()..].trim_start().starts_with(state))
        })
    })
}

fn preventive_adjacent_review_end(text: &str, start: usize) -> usize {
    let suffix = &text[start..];
    let section_blank = suffix
        .match_indices("\n\n")
        .map(|(index, _)| index)
        .find(|index| !is_preventive_adjacent_heading_blank(suffix, *index));
    [section_blank, suffix.find("\n#"), suffix.find("\nreview ")]
        .into_iter()
        .flatten()
        .min()
        .map_or(text.len(), |index| start + index)
}

fn is_preventive_adjacent_heading_blank(suffix: &str, index: usize) -> bool {
    suffix[..index]
        .trim_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '#' | ':' | '-' | '.'))
        == "preventive adjacent review"
}
