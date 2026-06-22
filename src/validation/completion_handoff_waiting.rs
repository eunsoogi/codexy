pub(super) fn check(handoff: &str) -> Option<String> {
    let text = handoff.to_ascii_lowercase();
    for segment in text.split(['\n', '.', ';']) {
        if claims_blocked_state(segment)
            && mentions_non_blocking_wait(segment)
            && !has_true_impasse_rationale(segment)
            && !mentions_true_blocker(segment)
        {
            return Some(
                "pending Codex review, child work, queued worktree/thread setup, and async tool completion are waiting state evidence, not blocked evidence"
                    .into(),
            );
        }
    }
    if claims_blocked_state(&text)
        && mentions_non_blocking_wait(&text)
        && !has_true_impasse_rationale(&text)
        && !mentions_true_blocker(&text)
    {
        return Some(
            "pending Codex review, child work, queued worktree/thread setup, and async tool completion are waiting state evidence, not blocked evidence"
                .into(),
        );
    }
    None
}

fn mentions_true_blocker(text: &str) -> bool {
    mentions_actionable_review_feedback(text)
        || mentions_missing_child_evidence(text)
        || mentions_setup_failure_blocker(text)
        || has_any(text, "security review")
}

fn claims_blocked_state(text: &str) -> bool {
    has_unnegated_word(text, "blocked", 16)
        || has_unnegated_word(text, "blocker", 16)
        || has_unnegated_word(text, "blockers", 16)
        || has_unnegated_phrase(text, "blocked state", 16)
        || has_unnegated_phrase(text, "goal blocked", 16)
}

fn mentions_non_blocking_wait(text: &str) -> bool {
    mentions_queued_setup(text)
        || mentions_async_completion(text)
        || mentions_return_wait(text)
        || (mentions_codex_review(text)
            && mentions_waiting_context(text)
            && !mentions_actionable_review_feedback(text))
        || (mentions_child_work(text)
            && mentions_waiting_context(text)
            && !mentions_missing_child_evidence(text))
}

fn mentions_codex_review(text: &str) -> bool {
    has_any(
        text,
        "@codex review|codex connector review|codex review|chatgpt-codex-connector",
    )
}

fn mentions_actionable_review_feedback(text: &str) -> bool {
    !has_any(
        text,
        "no actionable feedback|no feedback|no review feedback",
    ) && !mentions_pending_review_feedback_arrival(text)
        && (has_any(
            text,
            "feedback|requested changes|changes requested|suggestion|unresolved|actionable|resolution",
        ) || (has_any(text, "review comment|review comments")
            && !mentions_pending_request_context(text)))
}

fn mentions_pending_review_feedback_arrival(text: &str) -> bool {
    mentions_codex_review(text)
        && has_any(
            text,
            "waiting for codex review feedback|waiting for review feedback|codex review feedback from the connector|review feedback from the connector|feedback to arrive",
        )
}

fn mentions_pending_request_context(text: &str) -> bool {
    mentions_codex_review(text) && has_any(text, "eyes reaction|request")
}

fn mentions_child_work(text: &str) -> bool {
    has_any(
        text,
        "child-thread work|child thread work|child-thread|child thread|child work",
    )
}

fn mentions_queued_setup(text: &str) -> bool {
    has_any(text, "queued worktree|queued thread")
        || (has_any(text, "worktree setup|thread setup")
            && mentions_waiting_context(text)
            && !mentions_setup_failure(text))
}

fn mentions_setup_failure(text: &str) -> bool {
    has_any(
        text,
        "failed|failure|fatal|invalid reference|does not exist|missing",
    )
}

fn mentions_setup_failure_blocker(text: &str) -> bool {
    has_any(text, "worktree setup|thread setup") && mentions_setup_failure(text)
}

fn mentions_async_completion(text: &str) -> bool {
    has_any(text, "asynchronous tool|async tool")
        && has_any(text, "completion|pending|waiting|running|in progress")
}

fn mentions_return_wait(text: &str) -> bool {
    (mentions_codex_review(text) || mentions_child_work(text))
        && has_any(text, "until|waiting for")
        && has_any(
            text,
            "returns|return|comes back|responds|response|finishes|completes",
        )
        && !mentions_actionable_review_feedback(text)
        && !mentions_missing_child_evidence(text)
}

fn mentions_waiting_context(text: &str) -> bool {
    has_any(
        text,
        "pending|waiting|in progress|processing|eyes reaction|working",
    )
}

fn mentions_missing_child_evidence(text: &str) -> bool {
    mentions_child_work(text)
        && (has_any(text, "omitted|missing")
            || (has_any(text, "required|pending") && mentions_child_evidence_artifact(text)))
}

fn mentions_child_evidence_artifact(text: &str) -> bool {
    has_any(text, "evidence|goal tool|todo|plan|verification evidence")
}

fn has_true_impasse_rationale(text: &str) -> bool {
    (has_unnegated_phrase(text, "true impasse", 16)
        || has_unnegated_phrase(text, "cannot make meaningful progress", 16)
        || has_unnegated_phrase(text, "can't make meaningful progress", 16))
        && [
            "without user input",
            "without maintainer input",
            "without human input",
            "external state change",
            "requires user input",
            "requires maintainer input",
            "requires human input",
        ]
        .iter()
        .any(|phrase| has_unnegated_phrase(text, phrase, 16))
}

fn has_any(text: &str, phrases: &str) -> bool {
    phrases
        .split('|')
        .any(|phrase| has_unnegated_phrase(text, phrase, 16))
}

fn has_unnegated_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if phrase_has_boundaries(text, absolute_index, after_index) {
            let prefix_start = char_window_start(text, absolute_index, negation_window);
            if !has_nearby_negation(&text[prefix_start..absolute_index]) {
                return true;
            }
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}

fn has_unnegated_word(text: &str, word: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(word) {
        let absolute_index = offset + index;
        let after_index = absolute_index + word.len();
        if is_boundary(text[..absolute_index].chars().next_back())
            && is_boundary(text[after_index..].chars().next())
        {
            let prefix_start = char_window_start(text, absolute_index, negation_window);
            if !has_nearby_negation(&text[prefix_start..absolute_index])
                && !has_false_blocker_label(text, word, after_index)
            {
                return true;
            }
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}

fn has_false_blocker_label(text: &str, word: &str, after_index: usize) -> bool {
    if !matches!(word, "blocked" | "blocker" | "blockers") {
        return false;
    }
    let Some(value) = text[after_index..].trim_start().strip_prefix(':') else {
        return false;
    };
    ["none", "no", "false", "not applicable", "n/a", "na"]
        .iter()
        .any(|phrase| {
            value
                .trim_start()
                .strip_prefix(phrase)
                .is_some_and(|rest| is_boundary(rest.chars().next()))
        })
}

fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn has_nearby_negation(prefix: &str) -> bool {
    ["no", "not", "not a", "not an", "isn't", "is not", "without"]
        .iter()
        .any(|phrase| prefix.trim_end().ends_with(phrase))
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}
