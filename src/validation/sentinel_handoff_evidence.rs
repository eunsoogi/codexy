const MAINTAINER_FALLBACK_APPROVAL_MARKERS: &str = "maintainer explicitly approved fallback|maintainer explicitly approved a fallback|maintainer explicitly approved the fallback|maintainer approval: fallback approved|maintainer approval fallback approved";
const FALLBACK_REJECTION_MARKERS: &str = "fallback required|approval required|required before|no maintainer approval|no maintainer response|not approved|previous fallback|prior fallback|old fallback|earlier fallback|superseded fallback|previous unobservable|prior unobservable|old unobservable|earlier unobservable|superseded unobservable|previous sentinel|prior sentinel|old sentinel|earlier sentinel|superseded sentinel|previous codexy-sentinel|prior codexy-sentinel|old codexy-sentinel|earlier codexy-sentinel|superseded codexy-sentinel|previous reviewer gate|prior reviewer gate|old reviewer gate|earlier reviewer gate|superseded reviewer gate|previous reviewer-gate|prior reviewer-gate|old reviewer-gate|earlier reviewer-gate|superseded reviewer-gate";
const CURRENT_HEAD_REJECTION_MARKERS: &str = "old head|earlier head|previous head|prior head|stale head|old commit|previous commit|prior commit|stale commit|old sha|previous sha|prior sha|stale sha|old oid|previous oid|prior oid|stale oid|not on current head|not on current pr head|not on the current head|not on the current pr head|not for current head|not for current pr head|not for the current head|not for the current pr head|not current head|not current pr head";
const NEGATIVE_LABEL_VALUE_MARKERS: &str = "false|not ready|not yet ready|not currently ready|isn't ready|isn't yet ready|isn't currently ready|aren't ready|aren't yet ready|aren't currently ready|not requested|isn't requested|aren't requested|not applicable|isn't applicable|aren't applicable|missing|evidence missing|absent|not provided|n/a";
const AFFIRMATIVE_LABEL_VALUE_MARKERS: &str = "yes|true|approved|explicitly approved";

use super::sentinel_handoff::{affirmed_phrase_starts, clause_bounds, has_any, is_boundary};

pub(super) fn fallback_after(text: &str, start: usize) -> bool {
    let suffix = &text[start..];
    MAINTAINER_FALLBACK_APPROVAL_MARKERS
        .split('|')
        .any(|phrase| {
            affirmed_phrase_starts(suffix, phrase).any(|approval_start| {
                let approval_end = approval_start + phrase.len();
                let evidence_end = suffix[approval_start..]
                    .find(['.', '!', ';', '\n'])
                    .map(|offset| approval_start + offset)
                    .unwrap_or(suffix.len());
                let evidence = &suffix[approval_start..evidence_end];
                has_sentinel_fallback_target(&suffix[approval_end..evidence_end])
                    && !has_any(evidence, FALLBACK_REJECTION_MARKERS)
                    && !has_negative_answer(&suffix[approval_end..evidence_end])
                    && question_answers_are_affirmative(&suffix[approval_end..evidence_end])
            })
        })
}
fn has_sentinel_fallback_target(text: &str) -> bool {
    let text = text.trim_start_matches([' ', '\t', '-', ':']);
    let Some(target) = text.strip_prefix("for ") else {
        return false;
    };
    "this sentinel run|the sentinel run|current sentinel run|the current sentinel run|this unobservable sentinel run|this timed-out sentinel run|this timed out sentinel run|this codexy-sentinel run|this reviewer gate run|this reviewer-gate run|current reviewer gate run|current reviewer-gate run|the current reviewer gate run|the current reviewer-gate run"
        .split('|')
        .any(|phrase| target.strip_prefix(phrase).is_some_and(|rest| is_boundary(rest.chars().next())))
}

pub(super) fn names_head(text: &str, start: usize, head_ref_oid: Option<&str>) -> bool {
    let Some(head) = head_ref_oid.map(str::trim).filter(|head| !head.is_empty()) else {
        return false;
    };
    let bounds = clause_bounds(text, start);
    let evidence = &text[bounds.0..bounds.1];
    evidence.contains(&head.to_ascii_lowercase())
        && !has_any(evidence, CURRENT_HEAD_REJECTION_MARKERS)
}

fn has_negative_answer(text: &str) -> bool {
    has_negative_label_value(text)
        || text
            .split(['?', ':', '-', '='])
            .skip(1)
            .any(starts_with_negative_value)
}

fn question_answers_are_affirmative(text: &str) -> bool {
    text.split('?').skip(1).all(starts_with_affirmative_value)
}

pub(super) fn has_negative_label_value(suffix: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    starts_with_negative_value(value)
}

pub(super) fn has_non_claim_phrase_context(prefix: &str, suffix: &str) -> bool {
    has_unchecked_checklist_marker_before(prefix)
        || has_non_claim_heading_suffix(suffix)
        || has_non_claim_label_value(suffix)
        || has_missing_status_suffix(suffix)
}

fn has_missing_status_suffix(suffix: &str) -> bool {
    let clause = suffix
        .split(['.', '!', '?', ';', '\n'])
        .next()
        .unwrap_or_default();
    let words: Vec<_> = clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.windows(2).any(|pair| {
        matches!(
            pair,
            [
                "is" | "was" | "were" | "are" | "be" | "been",
                "missing" | "absent" | "lacking"
            ]
        )
    })
}

fn has_unchecked_checklist_marker_before(prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches([' ', '\t']);
    if prefix.ends_with("- [ ]") || prefix.ends_with("* [ ]") {
        return true;
    }
    let marker = prefix
        .rsplit_once('\n')
        .map_or(prefix, |(_, line)| line)
        .trim_end()
        .strip_suffix("[ ]")
        .map(str::trim_end);
    marker.is_some_and(|marker| {
        marker.strip_suffix(['.', ')']).is_some_and(|number| {
            !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit())
        })
    })
}

fn has_non_claim_heading_suffix(suffix: &str) -> bool {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    if suffix
        .strip_prefix("status")
        .is_some_and(|rest| is_boundary(rest.chars().next()) && has_non_claim_label_value(rest))
    {
        return true;
    }
    ["blocker", "blockers", "blocked", "pending", "waiting"]
        .iter()
        .any(|phrase| {
            suffix
                .strip_prefix(phrase)
                .is_some_and(|rest| is_boundary(rest.chars().next()))
        })
        || has_non_claim_next_line(suffix)
}

fn has_non_claim_next_line(suffix: &str) -> bool {
    let Some(rest) = suffix.strip_prefix('\n') else {
        return false;
    };
    let line = rest.lines().next().unwrap_or_default().trim_start();
    let item = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .unwrap_or(line);
    let item = item
        .split_once(['.', ')'])
        .filter(|(number, _)| !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit()))
        .map_or(item, |(_, rest)| rest)
        .trim_start();
    [
        "missing", "absent", "pending", "waiting", "blocked", "blocker",
    ]
    .iter()
    .any(|phrase| {
        item.strip_prefix(phrase)
            .is_some_and(|rest| is_boundary(rest.chars().next()))
    })
}

fn has_non_claim_label_value(suffix: &str) -> bool {
    if label_value(suffix).is_some_and(|value| {
        value
            .strip_prefix("no blockers")
            .is_some_and(|rest| is_boundary(rest.chars().next()))
    }) {
        return false;
    }
    has_negative_label_value(suffix)
        || [
            "not yet",
            "not currently",
            "unchecked",
            "missing",
            "unknown",
            "pending",
            "blocked",
            "waiting",
            "deferred",
            "none",
            "no",
        ]
        .iter()
        .any(|phrase| label_value_starts_with(suffix, phrase))
}

fn label_value_starts_with(suffix: &str, phrase: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    value
        .strip_prefix(phrase)
        .is_some_and(|rest| is_boundary(rest.chars().next()))
}

fn starts_with_negative_value(value: &str) -> bool {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']);
    if is_standalone_negative_no(value) {
        return true;
    }
    NEGATIVE_LABEL_VALUE_MARKERS.split('|').any(|phrase| {
        value
            .strip_prefix(phrase)
            .is_some_and(|rest| is_boundary(rest.chars().next()))
    })
}

fn starts_with_affirmative_value(value: &str) -> bool {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']);
    AFFIRMATIVE_LABEL_VALUE_MARKERS.split('|').any(|phrase| {
        value
            .strip_prefix(phrase)
            .is_some_and(|rest| is_boundary(rest.chars().next()))
    })
}

fn is_standalone_negative_no(value: &str) -> bool {
    let rest = value.strip_prefix("no");
    rest.is_some_and(|rest| {
        let rest = rest.trim_start_matches([' ', '\t']);
        rest.is_empty() || rest.starts_with(['.', ',', ';', '!', '?', '\n', '\r'])
    })
}

fn label_value(suffix: &str) -> Option<&str> {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let value = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('?'))?;
    Some(value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']))
}
