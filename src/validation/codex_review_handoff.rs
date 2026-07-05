use serde_json::Value;

use super::codex_review_handoff_actionable::has_actionable_codex_review_output;
use super::codex_review_handoff_events::{
    has_codex_review_activity, has_codex_review_output,
    has_latest_eyes_request_without_later_codex_output, has_unresolved_codex_review_thread,
};

const READY_PHRASES: &str = "merge-ready|merge ready|ready to merge|ready for merge|ready for parent handoff|pr-ready|pr ready|pull-request-ready|pull request ready|codex review passed|codex review completed|codex review complete|codex review approved";
const OVERRIDE_PHRASES: &str = "maintainer override: yes|maintainer override: granted|maintainer accepted proceeding without codex review|maintainer accepted proceeding without full codex review|maintainer explicitly accepted proceeding without codex review|maintainer explicitly accepted proceeding without full codex review";
pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let claims_ready = claims_codex_review_ready(handoff);
    let claims_completion = claims_codex_review_completion(handoff);
    let has_override = states_codex_review_override(handoff);
    if let Some(error) = super::codex_review_fresh_request::check(handoff, pr_state) {
        return vec![error];
    }
    if claims_ready
        && !has_head_ref_oid(pr_state)
        && (claims_completion || has_codex_review_activity(pr_state))
    {
        return vec![
            "completion handoff PR state missing required field: headRefOid before Codex review readiness claims"
                .into(),
        ];
    }
    if claims_ready && has_unresolved_codex_review_thread(pr_state) {
        return vec![format!(
            "unresolved Codex review thread blocks merge/readiness claims: PR #{}",
            pr_number(pr_state)
        )];
    }
    if claims_ready
        && has_actionable_codex_review_output(pr_state)
        && !super::review_thread_resolution::claims_review_response(handoff)
    {
        return vec![format!(
            "actionable Codex review output blocks merge/readiness claims until the handoff documents addressed feedback or an accepted no-change rationale: PR #{}",
            pr_number(pr_state)
        )];
    }
    if claims_ready && has_latest_eyes_request_without_later_codex_output(pr_state) && !has_override
    {
        return vec![format!(
            "eyes-only Codex review request or unacknowledged Codex review request is not review completion: PR #{} needs actual Codex review output, an explicit completion signal, or a maintainer override before merge/readiness claims",
            pr_number(pr_state)
        )];
    }
    if claims_ready && !has_codex_review_output(pr_state) && !has_override {
        return vec![format!(
            "current-head Codex review output is required before Codex review completion claims: PR #{} needs matching Codex review output or a maintainer override before merge/readiness claims",
            pr_number(pr_state)
        )];
    }
    if claims_ready
        && (has_codex_review_output(pr_state) || has_override && has_review_threads(pr_state))
    {
        if let Some(error) = review_thread_evidence_error(pr_state) {
            return vec![format!(
                "{error} before merge/readiness claims: PR #{}",
                pr_number(pr_state)
            )];
        }
        if let Some(error) = super::review_thread_readiness::check(pr_state) {
            return vec![format!("{error}: PR #{}", pr_number(pr_state))];
        }
    }
    Vec::new()
}
fn claims_codex_review_ready(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    READY_PHRASES
        .split('|')
        .any(|phrase| has_affirmed_phrase(&text, phrase))
}
fn claims_codex_review_completion(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    [
        "codex review passed",
        "codex review completed",
        "codex review complete",
        "codex review approved",
    ]
    .iter()
    .any(|phrase| has_affirmed_phrase(&text, phrase))
}
fn has_head_ref_oid(pr_state: &Value) -> bool {
    pr_state
        .get("headRefOid")
        .and_then(Value::as_str)
        .is_some_and(|head| !head.trim().is_empty())
}
fn states_codex_review_override(handoff: &str) -> bool {
    handoff.lines().any(|line| {
        let line = line.trim_start();
        let text = line.to_ascii_lowercase();
        let unordered = matches!(line.as_bytes().first(), Some(b'-' | b'*' | b'+'))
            && line[1..].trim_start().starts_with("[ ]");
        let ordered = line.split_once(['.', ')']).is_some_and(|(number, rest)| {
            !number.is_empty()
                && number.chars().all(|character| character.is_ascii_digit())
                && rest.trim_start().starts_with("[ ]")
        });
        !unordered
            && !ordered
            && OVERRIDE_PHRASES
                .split('|')
                .any(|phrase| has_affirmed_phrase(&text, phrase))
    })
}
fn review_thread_evidence_error(pr_state: &Value) -> Option<String> {
    let Some(threads) = pr_state.get("reviewThreads") else {
        return Some(
            "incomplete reviewThreads.nodes PR state evidence: missing reviewThreads".into(),
        );
    };
    if threads.get("nodes").and_then(Value::as_array).is_none() {
        return Some("incomplete reviewThreads.nodes PR state evidence: missing nodes".into());
    }
    super::review_thread_evidence::check(threads)
}
fn has_review_threads(pr_state: &Value) -> bool {
    pr_state.get("reviewThreads").is_some()
}
fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if is_boundary(text[..start].chars().next_back())
            && is_boundary(text[end..].chars().next())
            && !is_locally_negated(&text[..start])
            && !has_negative_label_value(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}
pub(super) fn has_negative_label_value(suffix: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    [
        "not ready",
        "not yet ready",
        "not currently ready",
        "isn't ready",
        "isn't yet ready",
        "isn't currently ready",
        "aren't ready",
        "aren't yet ready",
        "aren't currently ready",
        "false",
        "not requested",
        "isn't requested",
        "aren't requested",
        "not applicable",
        "isn't applicable",
        "aren't applicable",
    ]
    .iter()
    .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
        || value
            .strip_prefix("no")
            .is_some_and(starts_with_standalone_label_boundary)
}
fn label_value(suffix: &str) -> Option<&str> {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let value = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('?'))?;
    Some(value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']))
}
fn starts_with_boundary(rest: &str) -> bool {
    is_boundary(rest.chars().next())
}
fn starts_with_standalone_label_boundary(rest: &str) -> bool {
    rest.is_empty()
        || rest
            .chars()
            .next()
            .is_some_and(|character| matches!(character, '.' | ';' | ',' | '\n' | '\r'))
}
fn is_locally_negated(prefix: &str) -> bool {
    let clause_start = last_clause_boundary(prefix).map_or(0, |index| index);
    let clause = &prefix[clause_start..];
    clause
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| {
            matches!(
                word,
                "no" | "not"
                    | "never"
                    | "without"
                    | "isn't"
                    | "wasn't"
                    | "hasn't"
                    | "haven't"
                    | "aren't"
                    | "don't"
                    | "doesn't"
                    | "didn't"
                    | "won't"
                    | "can't"
                    | "cannot"
            )
        })
}
fn last_clause_boundary(text: &str) -> Option<usize> {
    let mut boundary = None;
    for (index, character) in text.char_indices() {
        let end = index + character.len_utf8();
        if matches!(character, '.' | '!' | '?' | ';' | ':' | ',' | '\n')
            || is_dash_separator(text, index, character)
        {
            boundary = Some(end);
        }
    }
    boundary
}
fn is_dash_separator(text: &str, index: usize, character: char) -> bool {
    if matches!(character, '–' | '—') {
        return true;
    }
    if character != '-' {
        return false;
    }
    let previous = text[..index].chars().next_back();
    let next = text[index + character.len_utf8()..].chars().next();
    previous.is_some_and(char::is_whitespace)
        && next.is_some_and(|character| character.is_whitespace() || character == '-')
}
fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}
fn pr_number(pr_state: &Value) -> String {
    pr_state
        .get("number")
        .and_then(Value::as_u64)
        .map_or_else(|| "<unknown>".to_owned(), |number| number.to_string())
}
