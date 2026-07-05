use serde_json::Value;

use super::codex_review_handoff_events::{
    has_codex_review_output, has_latest_eyes_request_without_later_codex_output,
};

pub(super) fn claims(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    text.lines().any(|line| {
        line.split([';', '.', '!', '?', ',']).any(|clause| {
            let clause = clause.trim();
            !has_negated_review_request(clause)
                && clause.contains("request")
                && (clause.contains("codex review") || clause.contains("@codex review"))
        })
    })
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

fn has_negated_review_request(clause: &str) -> bool {
    [
        "do not request",
        "don't request",
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
