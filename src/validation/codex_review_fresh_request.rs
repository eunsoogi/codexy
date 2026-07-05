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

pub(super) fn has_unresolved_actionable_thread(pr_state: &Value) -> bool {
    pr_state
        .get("reviewThreads")
        .and_then(|threads| threads.get("nodes"))
        .and_then(Value::as_array)
        .is_some_and(|nodes| {
            nodes.iter().any(|thread| {
                thread.get("isResolved").and_then(Value::as_bool) == Some(false)
                    && thread.get("isOutdated").and_then(Value::as_bool) != Some(true)
            })
        })
}

fn has_negated_review_request(clause: &str) -> bool {
    [
        "do not request",
        "don't request",
        "not request",
        "will not request",
        "won't request",
        "must not request",
        "mustn't request",
    ]
    .iter()
    .any(|phrase| clause.contains(phrase))
}
