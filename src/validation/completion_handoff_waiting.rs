const WAITING_STATE_ERROR: &str = "pending Codex review, child work, queued worktree/thread setup, and async tool completion are waiting state evidence, not blocked evidence";

pub(super) fn check(handoff: &str) -> Option<String> {
    let text = handoff.to_ascii_lowercase();
    let false_blocked_wait = |text: &str| {
        claims_blocked_state(text)
            && mentions_non_blocking_wait(text)
            && !has_true_impasse_rationale(text)
            && !mentions_true_blocker(text)
    };
    if text.split(['\n', '.', ';']).any(false_blocked_wait) || false_blocked_wait(&text) {
        return Some(WAITING_STATE_ERROR.into());
    }
    None
}

fn mentions_true_blocker(text: &str) -> bool {
    mentions_actionable_review_feedback(text)
        || mentions_missing_child_evidence(text)
        || mentions_setup_failure_blocker(text)
        || mentions_external_gate_blocker(text)
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
        && !has_any(
            text,
            "resolution|requested changes|changes requested|suggestion|unresolved|actionable",
        )
        && has_any(
            text,
            "pending codex review feedback|pending @codex review feedback|pending review feedback|codex review feedback is pending|@codex review feedback is pending|review feedback is pending|codex review is pending feedback from the connector|@codex review is pending feedback from the connector|waiting for codex review feedback|waiting for @codex review feedback|waiting for review feedback|codex review feedback from the connector|review feedback from the connector|feedback to arrive",
        )
}

fn mentions_external_gate_blocker(text: &str) -> bool {
    mentions_security_review_blocker(text)
        || has_any(
            text,
            "required status checks are failing|status checks are failing|status checks failed",
        )
}

fn mentions_security_review_blocker(text: &str) -> bool {
    has_any(
        text,
        "required security review|security review required|security review is required|pending security review|security review pending|security review is pending|security review failed|security review failure",
    ) && !has_any(
        text,
        "security review passed|security review complete|security review completed|security review not required|no security review",
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
    has_any(text, "asynchronous|async")
        && has_any(text, "tool")
        && has_any(
            text,
            "completion|pending|waiting|running|in progress|not returned|not yet returned|hasn't returned",
        )
}

fn mentions_return_wait(text: &str) -> bool {
    (mentions_codex_review(text) || mentions_child_work(text))
        && has_any(text, "until|waiting for")
        && has_any(
            text,
            "returns|return|returned|not returned|not yet returned|has not returned|hasn't returned|comes back|responds|response|finishes|completes",
        )
        && !mentions_actionable_review_feedback(text)
        && !mentions_missing_child_evidence(text)
}

fn mentions_waiting_context(text: &str) -> bool {
    has_any(
        text,
        "pending|waiting|in progress|processing|eyes reaction|working|not returned|not yet returned|has not returned|hasn't returned",
    )
}

fn mentions_missing_child_evidence(text: &str) -> bool {
    mentions_child_work(text)
        && has_any(text, "omitted|missing|required|pending")
        && has_any(text, "evidence|goal tool|todo|plan|verification evidence")
}

fn has_true_impasse_rationale(text: &str) -> bool {
    (has_unnegated_phrase(text, "true impasse", 16)
        || has_unnegated_phrase(text, "cannot make meaningful progress", 16)
        || has_unnegated_phrase(text, "can't make meaningful progress", 16))
        && has_any(
            text,
            "without user input|without maintainer input|without human input|external state change|requires user input|requires maintainer input|requires human input",
        )
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
    let value = text[after_index..].trim_start();
    if !matches!(value.chars().next(), Some(':' | '-' | '?')) {
        return false;
    }
    let value = &value[1..];
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
    "no|no known|non|non-|not|not a|not an|isn't|is not|without"
        .split('|')
        .any(|phrase| prefix.trim_end().ends_with(phrase))
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}
