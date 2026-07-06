use serde_json::Value;

use super::codex_review_handoff_events::{
    has_codex_review_output, has_latest_eyes_request_without_later_codex_output,
};

pub(super) fn claims(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    text.lines().any(|line| {
        line.split([';', '.', '!', ','])
            .flat_map(request_subclauses)
            .any(|clause| {
                let clause = clause.trim();
                !has_negated_review_request(clause)
                    && !super::codex_review_fresh_request_text::has_negative_request_status(clause)
                    && !super::codex_review_fresh_request_text::is_connector_footer(clause)
                    && !is_review_request_status_clause(clause)
                    && is_review_request_clause(clause)
            })
    })
}

fn is_review_request_clause(clause: &str) -> bool {
    let names_codex_review = clause.contains("codex review") || clause.contains("@codex review");
    let names_at_codex_review = clause.contains("@codex review");
    names_codex_review
        && (contains_word(clause, "request") && !is_pull_request_noun_clause(clause)
            || names_at_codex_review
                && ["post", "comment", "send"]
                    .iter()
                    .any(|verb| contains_word(clause, verb))
            || clause.trim() == "@codex review"
            || clause
                .trim_start()
                .strip_prefix("next action:")
                .is_some_and(|action| action.trim_start().starts_with("@codex review"))
            || clause.trim_start().starts_with("review request:"))
        || clause.contains("request review from @codex")
        || clause.contains("request @codex to review")
}

fn is_pull_request_noun_clause(clause: &str) -> bool {
    clause.contains("pull request") || clause.contains("pr request")
}

fn is_review_request_status_clause(clause: &str) -> bool {
    (clause.contains("@codex review request") || clause.contains("codex review request"))
        && [
            "request is pending",
            "request pending",
            "request has eyes",
            "request has eyes only",
            "request already has eyes",
            "request is waiting",
        ]
        .iter()
        .any(|status| clause.contains(status))
}

pub(super) fn request_subclauses(clause: &str) -> impl Iterator<Item = &str> {
    clause
        .split(" and ")
        .flat_map(|part| part.split(" then "))
        .flat_map(|part| part.split(" next action is to "))
}

fn contains_word(text: &str, word: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(word) {
        let start = offset + index;
        let end = start + word.len();
        if text[..start]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && text[end..]
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric())
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

pub(super) fn check(handoff: &str, pr_state: &Value) -> Option<String> {
    if !claims(handoff) {
        return None;
    }
    if let Some(error) = review_thread_evidence_error(pr_state) {
        return Some(format!(
            "{error} before fresh Codex review requests: PR #{}",
            pr_number(pr_state)
        ));
    }
    if pr_state
        .get("headRefOid")
        .and_then(Value::as_str)
        .is_none_or(|head| head.trim().is_empty())
    {
        return Some(format!(
            "incomplete headRefOid PR state evidence before fresh Codex review requests: PR #{}",
            pr_number(pr_state)
        ));
    }
    if has_latest_eyes_request_without_later_codex_output(pr_state)
        || has_codex_review_output(pr_state)
    {
        return Some(format!(
            "current-head Codex review activity blocks fresh Codex review requests: PR #{} already has current-head request/output evidence",
            pr_number(pr_state)
        ));
    }
    if has_blocking_unresolved_thread(handoff, pr_state) {
        return Some(format!(
            "unresolved review thread blocks fresh Codex review requests: PR #{} must resolve or document accepted no-change rationale before requesting another @codex review",
            pr_number(pr_state)
        ));
    }
    None
}

pub(super) fn review_thread_evidence_error(pr_state: &Value) -> Option<String> {
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

pub(super) fn has_blocking_unresolved_thread(handoff: &str, pr_state: &Value) -> bool {
    pr_state
        .get("reviewThreads")
        .and_then(|threads| threads.get("nodes"))
        .and_then(Value::as_array)
        .is_some_and(|nodes| {
            nodes.iter().any(|thread| {
                thread.get("isResolved").and_then(Value::as_bool) == Some(false)
                    && thread.get("isOutdated").and_then(Value::as_bool) != Some(true)
                    && !super::review_thread_resolution::documents_accepted_no_change_rationale(
                        handoff, thread,
                    )
            })
        })
}

fn pr_number(pr_state: &Value) -> String {
    pr_state
        .get("number")
        .and_then(Value::as_u64)
        .map_or_else(|| "<unknown>".to_owned(), |number| number.to_string())
}

pub(super) fn has_negated_review_request(clause: &str) -> bool {
    if clause
        .rsplit_once(':')
        .is_some_and(|(_, action)| is_post_colon_request_action(action.trim()))
    {
        return false;
    }
    [
        "do not request",
        "don't request",
        "do not post",
        "don't post",
        "do not comment",
        "don't comment",
        "do not send",
        "don't send",
        "before requesting",
        "@codex review requested",
        "codex review requested",
        "requested a @codex review",
        "requested a codex review",
        "requested @codex review",
        "requested codex review",
        "no @codex review request",
        "no codex review request",
        "no current-head @codex review request",
        "no current-head codex review request",
        "no current-head request",
        "no current head @codex review request",
        "no current head codex review request",
        "no current head request",
        "no request",
        "without @codex review request",
        "without codex review request",
        "without current-head @codex review request",
        "without current-head codex review request",
        "not request",
        "without current-head request",
        "without current head @codex review request",
        "without current head codex review request",
        "without current head request",
        "without request",
        "will not request",
        "won't request",
        "must not request",
        "mustn't request",
        "will not post",
        "won't post",
        "must not post",
        "mustn't post",
        "will not comment",
        "won't comment",
        "must not comment",
        "mustn't comment",
        "will not send",
        "won't send",
        "must not send",
        "mustn't send",
        "not ready to request",
        "not yet ready to request",
        "not currently ready to request",
        "isn't ready to request",
        "isn't yet ready to request",
        "isn't currently ready to request",
        "aren't ready to request",
        "aren't yet ready to request",
        "aren't currently ready to request",
    ]
    .iter()
    .any(|phrase| clause.contains(phrase))
}

fn is_post_colon_request_action(action: &str) -> bool {
    (action.starts_with("request ")
        || action.starts_with("post ")
        || action.starts_with("comment ")
        || action.starts_with("send ")
        || action.starts_with("@codex review"))
        && is_review_request_clause(action)
}
