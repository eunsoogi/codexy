pub(super) fn has_review_request_context(line: &str) -> bool {
    if has_follow_up_review_request_context(line) {
        return true;
    }

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
            ) || has_actionable_codex_review_request_context(clause))
    })
}

pub(super) fn has_codex_review_request_context(line: &str) -> bool {
    if has_follow_up_review_request_context(line) {
        return true;
    }

    line.split([';', '.', '!', '?', ',']).any(|clause| {
        let clause = clause.trim();
        !has_negated_review_request_context(clause)
            && !has_wait_only_review_output_context(clause)
            && (has_any(
                clause,
                &[
                    "@codex review",
                    "request codex review",
                    "request a codex review",
                    "request fresh codex review",
                    "request a fresh codex review",
                ],
            ) || has_actionable_codex_review_request_context(clause))
    })
}

fn has_actionable_codex_review_request_context(line: &str) -> bool {
    line.split([';', '.', '!', '?', ','])
        .flat_map(super::super::codex_review_fresh_request::request_subclauses)
        .map(str::trim)
        .any(|clause| {
            super::super::codex_review_fresh_request::is_review_request_clause(clause)
                && !super::super::codex_review_fresh_request::has_negated_review_request(clause)
                && !super::super::codex_review_fresh_request_text::is_connector_footer(clause)
                && !super::super::codex_review_fresh_request_text::has_negative_request_status(
                    clause,
                )
        })
}

fn has_follow_up_review_request_context(line: &str) -> bool {
    for sentence in line.split(['.', '!', '?', '\n']) {
        if has_follow_up_review_request_context_in_sentence(sentence) {
            return true;
        }
    }
    false
}

fn has_follow_up_review_request_context_in_sentence(sentence: &str) -> bool {
    let mut has_review_wait_context = false;
    for clause in sentence.split([';', ',']) {
        let clause = clause.trim();
        if has_wait_only_review_output_context(clause) {
            has_review_wait_context = true;
            continue;
        }
        if !has_negated_follow_up_review_request_context(clause)
            && (has_explicit_follow_up_review_request_context(clause)
                || has_review_wait_context && has_implicit_follow_up_review_request_phrase(clause))
        {
            return true;
        }
    }
    false
}

fn has_explicit_follow_up_review_request_context(line: &str) -> bool {
    has_follow_up_request_phrase(line)
        && (line.contains("codex review") || line.contains("@codex review"))
}

fn has_implicit_follow_up_review_request_phrase(line: &str) -> bool {
    has_any(
        line,
        &[
            "request again",
            "request another review",
            "request a new review",
            "request new review",
            "request fresh review",
            "request a fresh review",
        ],
    )
}

fn has_negated_actionable_codex_review_request_context(line: &str) -> bool {
    (line.contains("codex review") || line.contains("@codex"))
        && has_any(
            line,
            &[
                "do not request",
                "don't request",
                "not request",
                "will not request",
                "won't request",
                "must not request",
                "without posting",
                "without requesting",
            ],
        )
}

fn has_follow_up_request_phrase(line: &str) -> bool {
    has_any(
        line,
        &[
            "request again",
            "request another",
            "request a new",
            "request new",
            "request fresh",
            "request a fresh",
        ],
    )
}

fn has_negated_follow_up_review_request_context(line: &str) -> bool {
    has_any(
        line,
        &[
            "do not request again",
            "do not request another",
            "do not request fresh",
            "do not request a fresh",
            "do not request new",
            "do not request a new",
            "don't request again",
            "don't request another",
            "don't request fresh",
            "don't request a fresh",
            "don't request new",
            "don't request a new",
            "must not request again",
            "must not request another",
            "must not request fresh",
            "must not request a fresh",
            "must not request new",
            "must not request a new",
            "not request again",
            "not request another",
            "not request fresh",
            "not request a fresh",
            "not request new",
            "not request a new",
            "will not request again",
            "will not request another",
            "will not request fresh",
            "will not request a fresh",
            "will not request new",
            "will not request a new",
            "won't request again",
            "won't request another",
            "won't request fresh",
            "won't request a fresh",
            "won't request new",
            "won't request a new",
        ],
    )
}

fn has_wait_only_review_output_context(line: &str) -> bool {
    has_any(line, &["wait", "waiting", "poll", "polling"])
        && has_any(
            line,
            &[
                "@codex review",
                "codex review",
                "@codex review output",
                "codex review output",
                "codex review result",
                "codex review response",
            ],
        )
        && !has_any(
            line,
            &[
                "post @codex review",
                "post codex review",
                "request @codex review",
                "request codex review",
                "request a codex review",
                "request fresh codex review",
                "request a fresh codex review",
            ],
        )
}

fn has_negated_review_request_context(line: &str) -> bool {
    has_any(
        line,
        &[
            "not ready for review",
            "no @codex review request",
            "no codex review request",
            "no current-head @codex review request",
            "no current-head codex review request",
            "no current head @codex review request",
            "no current head codex review request",
            "no active @codex review request",
            "no active codex review request",
            "no active current-head @codex review request",
            "no active current-head codex review request",
            "no active current head @codex review request",
            "no active current head codex review request",
            "no review request",
            "without @codex review request",
            "without codex review request",
            "without review request",
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
    ) || has_negated_actionable_codex_review_request_context(line)
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}
