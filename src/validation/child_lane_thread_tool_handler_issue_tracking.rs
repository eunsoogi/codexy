use super::child_lane_thread_tool_handler_issue_reference::has_issue_reference;
use super::child_lane_thread_tool_handler_issue_value::has_placeholder_or_pending_value;

pub(super) fn has_tracking_issue(evidence: &str) -> bool {
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

fn handoff_clauses(evidence: &str) -> impl Iterator<Item = &str> {
    evidence
        .split(['\n', ';'])
        .flat_map(|clause| clause.split(". "))
        .map(str::trim)
}

fn has_negated_tracking_issue(clause: &str) -> bool {
    const NEGATED_TRACKING_ISSUE_MARKERS: &str = "no separate dogfood issue|no separate dogfooding issue|no issue,|no issue #|no separate issue|no issue was created|no issue created|no issue has been created|no issue filed|no issue was filed|no issue has been filed|no separate tracking issue|no tracking issue|no follow-up issue|no separate follow-up issue|not provided|not a tracking issue|not a separate tracking issue|not a dogfood issue|not a separate dogfood issue|not a dogfooding issue|not a separate dogfooding issue|not a follow-up issue|not a separate follow-up issue|without a separate dogfood issue|without a separate dogfooding issue|without a separate tracking issue|without tracking issue|without a follow-up issue|without follow-up issue";
    NEGATED_TRACKING_ISSUE_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker))
        || has_negated_issue_lifecycle(clause)
}

fn has_negated_issue_lifecycle(clause: &str) -> bool {
    let normalized = clause
        .replace("wasn't", "was not")
        .replace("hasn't", "has not")
        .replace("hadn't", "had not")
        .replace("isn't", "is not")
        .replace("didn't", "did not")
        .replace("won't", "will not");
    ["was", "has", "had", "is"].into_iter().any(|auxiliary| {
        ["created", "filed"].into_iter().any(|verb| {
            normalized.contains(&format!("issue {auxiliary} not been {verb}"))
                || normalized.contains(&format!("issue {auxiliary} not yet been {verb}"))
                || normalized.contains(&format!("issue {auxiliary} not {verb}"))
                || normalized.contains(&format!("issue {auxiliary} not yet {verb}"))
        })
    }) || ["created", "filed", "opened"].into_iter().any(|verb| {
        normalized.contains(&format!("issue not {verb}"))
            || normalized.contains(&format!("issue not yet {verb}"))
            || normalized.contains(&format!("issue did not {verb}"))
            || normalized.contains(&format!("issue did not get {verb}"))
            || lifecycle_verb_stem(verb)
                .is_some_and(|stem| normalized.contains(&format!("issue did not {stem}")))
            || has_issue_reference_with_lifecycle_negation(&normalized, verb)
    })
}

fn lifecycle_verb_stem(verb: &str) -> Option<&str> {
    match verb {
        "created" => Some("create"),
        "filed" => Some("file"),
        "opened" => Some("open"),
        _ => None,
    }
}

fn has_issue_reference_with_lifecycle_negation(clause: &str, verb: &str) -> bool {
    clause.match_indices('#').any(|(index, _)| {
        let tail = &clause[index + 1..];
        let digit_end = tail
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(tail.len());
        if digit_end == 0 {
            return false;
        }
        let after_reference = tail[digit_end..].trim_start_matches(|character: char| {
            character.is_ascii_whitespace()
                || matches!(
                    character,
                    ':' | '=' | '-' | '\u{2013}' | '\u{2014}' | ',' | ';' | '('
                )
        });
        has_lifecycle_negation_prefix(after_reference, verb)
    }) || clause.match_indices("/issues/").any(|(index, marker)| {
        let tail = &clause[index + marker.len()..];
        let digit_end = tail
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(tail.len());
        if digit_end == 0 {
            return false;
        }
        let after_reference = tail[digit_end..].trim_start_matches(|character: char| {
            character.is_ascii_whitespace()
                || matches!(
                    character,
                    '/' | ':' | '=' | '-' | '\u{2013}' | '\u{2014}' | ',' | ';' | '.' | '('
                )
        });
        has_lifecycle_negation_prefix(after_reference, verb)
    })
}

fn has_lifecycle_negation_prefix(after_reference: &str, verb: &str) -> bool {
    let after_reference = trim_lifecycle_connector_prefix(after_reference);
    [
        format!("not {verb}"),
        format!("not yet {verb}"),
        format!("not {verb} yet"),
        format!("was not {verb}"),
        format!("was not yet {verb}"),
        format!("was not {verb} yet"),
        format!("is not {verb}"),
        format!("is not yet {verb}"),
        format!("is not {verb} yet"),
        format!("has not been {verb}"),
        format!("has not yet been {verb}"),
        format!("has not been {verb} yet"),
        format!("will be {verb}"),
        format!("will not be {verb}"),
        format!("to be {verb}"),
        format!("should be {verb}"),
        format!("needs to be {verb}"),
    ]
    .into_iter()
    .any(|prefix| {
        after_reference == prefix
            || after_reference.strip_prefix(&prefix).is_some_and(|rest| {
                rest.starts_with(|character: char| !character.is_ascii_alphanumeric())
            })
    })
}

fn trim_lifecycle_connector_prefix(mut value: &str) -> &str {
    loop {
        let trimmed = value
            .trim_start()
            .trim_start_matches([',', ';', ':', '-', '\u{2013}', '\u{2014}'])
            .trim_start();
        let Some((connector, rest)) = trimmed.split_once(' ') else {
            return trimmed;
        };
        if !["and", "but", "however", "though", "although", "yet"]
            .into_iter()
            .any(|allowed| connector == allowed)
        {
            return trimmed;
        }
        value = rest;
    }
}
