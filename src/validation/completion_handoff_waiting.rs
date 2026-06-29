const WAITING_STATE_ERROR: &str = "pending Codex review, child work, queued worktree/thread setup, and async tool completion are waiting state evidence, not blocked evidence";
const SETUP_FAILURE: &str = "failed|failure|fatal|invalid reference|does not exist|missing";
const CODEX_REVIEW: &str = "codex review|codex connector review|chatgpt-codex-connector";
const NO_ACTIONABLE_REVIEW_FEEDBACK: &str = "no actionable feedback|no feedback|no review feedback";
const ACTIONABLE_REVIEW_FEEDBACK: &str =
    "feedback|requested changes|changes requested|suggestion|unresolved|actionable|resolution";
const PENDING_REVIEW_FEEDBACK: &str = "pending codex review feedback|pending @codex review feedback|pending codex connector review feedback|codex review feedback is pending|@codex review feedback is pending|codex connector review feedback is pending|codex review feedback pending|@codex review feedback pending|codex connector review feedback pending|codex review feedback has not returned|@codex review feedback has not returned|codex connector review feedback has not returned|codex review feedback has not yet returned|@codex review feedback has not yet returned|codex connector review feedback has not yet returned|codex review is pending feedback from the connector|@codex review is pending feedback from the connector|codex connector review is pending feedback from the connector|waiting on codex review feedback|waiting on @codex review feedback|waiting on codex connector review feedback|codex review is waiting on feedback|@codex review is waiting on feedback|codex connector review is waiting on feedback|pending codex review, waiting on feedback|pending @codex review, waiting on feedback|pending codex connector review, waiting on feedback|waiting for codex review feedback|waiting for @codex review feedback|waiting for codex connector review feedback|codex review is waiting for feedback|@codex review is waiting for feedback|codex connector review is waiting for feedback|pending codex review, waiting for feedback|pending @codex review, waiting for feedback|pending codex connector review, waiting for feedback|awaiting codex review feedback|awaiting @codex review feedback|awaiting codex connector review feedback|codex review is awaiting feedback|@codex review is awaiting feedback|codex connector review is awaiting feedback|pending codex review, awaiting feedback|pending @codex review, awaiting feedback|pending codex connector review, awaiting feedback|codex review feedback from the connector|codex connector review feedback from the connector|codex review feedback to arrive|@codex review feedback to arrive|codex connector review feedback to arrive";
const EXTERNAL_CHECK_FAILURE: &str = "required checks are failing|required checks failed|required status checks are failing|status checks are failing|status checks failed";
const SECURITY_REVIEW_BLOCKER: &str = "required security review|security review required|security review is required|pending security review|security review pending|security review is pending|security review failed|security review failure";
const SECURITY_REVIEW_NON_BLOCKER: &str = "security review passed|security review complete|security review completed|security review not required|no security review";
const CHILD_WORK: &str = "child-owned|review-response work|child-lane|child lane|child-thread work|child thread work|child-thread|child thread|child work";

pub(super) fn check(handoff: &str) -> Option<String> {
    let text = handoff.to_ascii_lowercase();
    let false_blocked_wait = |fragment: &str, context: &str| {
        claims_blocked_state(fragment)
            && mentions_non_blocking_wait(fragment)
            && !has_true_impasse_rationale(fragment)
            && !mentions_true_blocker(fragment)
            && !mentions_current_true_blocker_context(context)
            && !mentions_returned_async_failure_context(fragment, &text)
    };
    if text.split(['\n', '.']).any(|context| {
        context
            .split([',', ';'])
            .any(|fragment| false_blocked_wait(fragment, context))
    }) || false_blocked_wait(&text, &text)
    {
        return Some(WAITING_STATE_ERROR.into());
    }
    None
}
fn mentions_true_blocker(text: &str) -> bool {
    mentions_actionable_review_feedback(text)
        || mentions_missing_child_evidence(text)
        || (has_any(text, "worktree setup|thread setup") && has_any(text, SETUP_FAILURE))
        || mentions_external_gate_blocker(text)
}
fn mentions_current_true_blocker_context(text: &str) -> bool {
    text.split([',', ';'])
        .any(|part| mentions_true_blocker(part) && !mentions_resolved_blocker(part))
}
fn mentions_resolved_blocker(text: &str) -> bool {
    has_any(
        text,
        "blocker resolved|previous blocker resolved|resolved blocker",
    ) || has_any(
        text,
        "required checks failed and were fixed|required checks failed and were resolved|required checks failed and were cleared|required status checks failed and were fixed|required status checks failed and were resolved|required status checks failed and were cleared|status checks failed and were fixed|status checks failed and were resolved|status checks failed and were cleared|required checks were fixed|required checks were resolved|required checks were cleared|required status checks were fixed|required status checks were resolved|required status checks were cleared|status checks were fixed|status checks were resolved|status checks were cleared",
    )
}
fn claims_blocked_state(text: &str) -> bool {
    has_unnegated_word(text, "blocked", 16)
        || has_unnegated_word(text, "blocker", 16)
        || has_unnegated_word(text, "blockers", 16)
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
    has_any(text, CODEX_REVIEW)
}
fn mentions_actionable_review_feedback(text: &str) -> bool {
    !has_any(text, NO_ACTIONABLE_REVIEW_FEEDBACK)
        && !mentions_pending_review_feedback_arrival(text)
        && (mentions_codex_review(text)
            || has_any(text, "review|requested changes|changes requested"))
        && (has_any(text, ACTIONABLE_REVIEW_FEEDBACK)
            || (has_any(text, "review comment|review comments")
                && !(mentions_codex_review(text) && has_any(text, "eyes reaction|request"))))
}
fn mentions_pending_review_feedback_arrival(text: &str) -> bool {
    mentions_codex_review(text)
        && !has_any(
            text,
            "resolution|requested changes|changes requested|suggestion|unresolved|actionable",
        )
        && has_any(text, PENDING_REVIEW_FEEDBACK)
}
fn mentions_external_gate_blocker(text: &str) -> bool {
    (has_any(text, SECURITY_REVIEW_BLOCKER) && !has_any(text, SECURITY_REVIEW_NON_BLOCKER))
        || has_any(text, EXTERNAL_CHECK_FAILURE)
}
fn mentions_child_work(text: &str) -> bool {
    has_any(text, CHILD_WORK)
}
fn mentions_queued_setup(text: &str) -> bool {
    has_any(text, "queued worktree|queued thread")
        || (has_any(text, "worktree setup|thread setup")
            && (mentions_waiting_context(text) || has_any(text, "queued"))
            && !has_any(text, SETUP_FAILURE))
}
fn mentions_async_completion(text: &str) -> bool {
    mentions_async_tool_result(text)
        && has_any(
            text,
            "completion|pending|waiting|running|in progress|not returned|not yet returned|has not returned|hasn't returned|to return|until",
        )
        && !mentions_returned_async_failure(text)
}
fn mentions_returned_async_failure(text: &str) -> bool {
    mentions_async_tool_result(text)
        && has_any(text, "returned")
        && has_any(text, "error|failure|failed|permission|authentication|fatal")
}
fn mentions_returned_async_failure_context(fragment: &str, text: &str) -> bool {
    mentions_returned_async_failure(fragment)
        || (mentions_async_tool_result(fragment)
            && has_any(fragment, "returned")
            && has_any(text, "error|failure|failed|permission|authentication|fatal"))
}
fn mentions_async_tool_result(text: &str) -> bool {
    (has_any(text, "asynchronous|async") && has_any(text, "tool|operation|result"))
        || has_any(text, "tool result|background operation")
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
        "pending|waiting|awaiting|in progress|processing|eyes reaction|working|not returned|not yet returned|has not returned|hasn't returned",
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
    let value = value.strip_prefix("state").unwrap_or(value).trim_start();
    if !matches!(value.chars().next(), Some(':' | '-' | '?')) {
        return false;
    }
    let value = value[1..].trim_start();
    "none|no|false|not applicable|n/a|na"
        .split('|')
        .any(|phrase| {
            value
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
    let prefix = prefix.trim_end();
    negation_phrase_matches(prefix)
        || prefix.rsplit_once(' ').is_some_and(|(before, word)| {
            matches!(
                word,
                "actually" | "currently" | "presently" | "still" | "yet"
            ) && negation_phrase_matches(before)
        })
}

fn negation_phrase_matches(prefix: &str) -> bool {
    "no|no known|non|non-|not|not a|not an|isn't|is not|hasn't|without"
        .split('|')
        .any(|phrase| prefix.ends_with(phrase))
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}
