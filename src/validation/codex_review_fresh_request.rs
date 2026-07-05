use serde_json::Value;

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

fn has_negated_review_request(clause: &str) -> bool {
    [
        "do not request",
        "don't request",
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
    ]
    .iter()
    .any(|phrase| clause.contains(phrase))
}
