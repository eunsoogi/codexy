pub(super) fn has_review_request_context(line: &str) -> bool {
    line.split([';', '.', '!', '?', ',']).any(|clause| {
        let clause = clause.trim();
        !has_negated_review_request_context(clause)
            && !has_wait_only_review_output_context(clause)
            && (has_any(
                clause,
                &[
                    "review request",
                    "ready for review",
                    "@codex review",
                    "request codex review",
                    "request a codex review",
                    "request fresh codex review",
                    "request a fresh codex review",
                ],
            ) || has_request_codex_review_context(clause))
    })
}

fn has_request_codex_review_context(line: &str) -> bool {
    line.contains("request") && (line.contains("codex review") || line.contains("@codex review"))
}

fn has_wait_only_review_output_context(line: &str) -> bool {
    has_any(line, &["wait", "waiting", "poll", "polling"])
        && has_any(
            line,
            &[
                "@codex review output",
                "codex review output",
                "codex review result",
                "codex review response",
            ],
        )
        && !has_any(line, &["request", "post"])
}

fn has_negated_review_request_context(line: &str) -> bool {
    has_any(
        line,
        &[
            "not ready for review",
            "no @codex review request",
            "no codex review request",
            "no active @codex review request",
            "no active codex review request",
            "no review request",
            "without @codex review request",
            "without codex review request",
            "without review request",
            "do not request codex review",
            "don't request codex review",
            "not request codex review",
            "will not request codex review",
            "won't request codex review",
            "do not post @codex review",
            "don't post @codex review",
            "must not post @codex review",
            "not post @codex review",
            "will not post @codex review",
            "won't post @codex review",
            "do not post codex review",
            "don't post codex review",
            "must not post codex review",
            "not post codex review",
            "will not post codex review",
            "won't post codex review",
        ],
    ) || has_negated_request_codex_review_context(line)
}

fn has_negated_request_codex_review_context(line: &str) -> bool {
    has_request_codex_review_context(line)
        && has_any(
            line,
            &[
                "do not request",
                "don't request",
                "not request",
                "will not request",
                "won't request",
            ],
        )
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}
