const WAITING_STATE_ERROR: &str = "pending child work, queued worktree/thread setup, and async tool completion are waiting state evidence, not blocked evidence";
pub(crate) mod readiness_status;

mod matching;
mod phrases;

use super::child_goal_blocked_audit::wait_taxonomy::{
    WaitDisposition, classify_reviewer_text, classify_wait_text,
};
use matching::{has_any, has_unnegated_word};
use phrases::{
    blocked::{
        CURRENT_CLAIM, DISALLOWED_RATIONALE, MATERIAL_CHANGE, NO_IN_SCOPE_ACTION, NO_SAFE_DEFAULT,
        USER_DECISION,
    },
    checks::{
        EXTERNAL_FAILURE, FAILURE_CONTEXT, FAILURE_WORDS, FALSE_CHECK_LABEL, RESOLVED_BLOCKER,
        RESOLVED_CHECK,
    },
    review::{BARE_CONNECTOR_PASS, NONTERMINAL_GATE, SECURITY_BLOCKER, SECURITY_NON_BLOCKER},
    setup::{CONTEXT as SETUP_CONTEXT, FAILURE as SETUP_FAILURE, QUEUED, QUEUED_MARKER},
    waiting::{
        ASYNC_COMPLETION, ASYNC_FAILURE, ASYNC_MARKER, ASYNC_RESULT, ASYNC_SUBJECT, ASYNC_WAIT,
        CHILD_WORK, EVIDENCE_MARKER, HISTORICAL_ASYNC_FAILURE, MISSING_MARKER, RETURN_WAIT_RESULT,
        RETURN_WAIT_TRIGGER, RETURNED, WAITING_CONTEXT,
    },
};

pub(super) fn check(handoff: &str) -> Option<String> {
    let text = handoff.to_ascii_lowercase();
    if let Some(error) = super::completion_handoff_pending_worktree::check(&text) {
        return Some(error);
    }
    let false_blocked_wait = |fragment: &str, context: &str| {
        !readiness_status::is_neutral_heading(fragment)
            && claims_blocked_state(fragment)
            && mentions_non_blocking_wait(fragment)
            && !has_true_impasse_rationale(fragment)
            && (!mentions_resolved_blocker(fragment)
                || fragment.trim().starts_with("blocked")
                || has_any(fragment, CURRENT_CLAIM))
            && !mentions_true_blocker(fragment)
            && !context
                .split([',', ';'])
                .any(|part| mentions_true_blocker(part) && !mentions_resolved_blocker(part))
            && !mentions_returned_async_failure_context(fragment, &text)
    };
    let has_neutral_readiness_status = text
        .split(['\n', '.'])
        .any(readiness_status::is_neutral_heading);
    if text.split(['\n', '.']).any(|context| {
        context
            .split([',', ';'])
            .any(|fragment| false_blocked_wait(fragment, context))
    }) || (false_blocked_wait(&text, &text) && !has_neutral_readiness_status)
    {
        return Some(WAITING_STATE_ERROR.into());
    }
    None
}

fn mentions_true_blocker(text: &str) -> bool {
    mentions_actionable_review_feedback(text)
        || mentions_missing_child_evidence(text)
        || (has_any(text, SETUP_CONTEXT) && has_any(text, SETUP_FAILURE))
        || mentions_external_gate_blocker(text)
        || mentions_async_tool_failure(text)
}

fn mentions_resolved_blocker(text: &str) -> bool {
    !has_any(text, FAILURE_CONTEXT)
        && (has_any(text, RESOLVED_BLOCKER) || has_any(text, RESOLVED_CHECK))
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
        || classify_wait_text(text) == Some(WaitDisposition::Nonterminal)
        || has_any(text, BARE_CONNECTOR_PASS)
        || has_any(text, DISALLOWED_RATIONALE)
        || (has_any(text, NONTERMINAL_GATE) && mentions_waiting_context(text))
        || (has_any(text, CHILD_WORK)
            && mentions_waiting_context(text)
            && !mentions_missing_child_evidence(text))
}

fn mentions_actionable_review_feedback(text: &str) -> bool {
    classify_wait_text(text) == Some(WaitDisposition::Actionable)
}

fn mentions_external_gate_blocker(text: &str) -> bool {
    (has_any(text, SECURITY_BLOCKER)
        && !has_any(text, SECURITY_NON_BLOCKER)
        && (has_any(text, FAILURE_WORDS)
            || classify_reviewer_text(text) != Some(WaitDisposition::Nonterminal)))
        || (has_any(text, EXTERNAL_FAILURE)
            && !has_any(text, FALSE_CHECK_LABEL)
            && !mentions_resolved_blocker(text))
}

fn mentions_queued_setup(text: &str) -> bool {
    has_any(text, QUEUED)
        || (has_any(text, SETUP_CONTEXT)
            && (mentions_waiting_context(text) || has_any(text, QUEUED_MARKER))
            && !has_any(text, SETUP_FAILURE))
}

fn mentions_async_completion(text: &str) -> bool {
    mentions_async_tool_result(text)
        && has_any(text, ASYNC_COMPLETION)
        && !mentions_returned_async_failure(text)
        && !mentions_async_tool_failure(text)
}

fn mentions_async_tool_failure(text: &str) -> bool {
    mentions_async_tool_result(text)
        && has_any(text, ASYNC_FAILURE)
        && !has_any(text, RETURNED)
        && !has_any(text, ASYNC_WAIT)
}

fn mentions_returned_async_failure(text: &str) -> bool {
    mentions_async_tool_result(text) && has_any(text, RETURNED) && has_any(text, ASYNC_FAILURE)
}

fn mentions_returned_async_failure_context(fragment: &str, text: &str) -> bool {
    (mentions_returned_async_failure(fragment) && !has_any(fragment, HISTORICAL_ASYNC_FAILURE))
        || (mentions_returned_async_failure(text) && !has_any(text, HISTORICAL_ASYNC_FAILURE))
}

fn mentions_async_tool_result(text: &str) -> bool {
    (has_any(text, ASYNC_MARKER) && has_any(text, ASYNC_SUBJECT)) || has_any(text, ASYNC_RESULT)
}

fn mentions_return_wait(text: &str) -> bool {
    has_any(text, CHILD_WORK)
        && has_any(text, RETURN_WAIT_TRIGGER)
        && has_any(text, RETURN_WAIT_RESULT)
        && !mentions_actionable_review_feedback(text)
        && !mentions_missing_child_evidence(text)
}

fn mentions_waiting_context(text: &str) -> bool {
    has_any(text, WAITING_CONTEXT)
}

fn mentions_missing_child_evidence(text: &str) -> bool {
    has_any(text, CHILD_WORK) && has_any(text, MISSING_MARKER) && has_any(text, EVIDENCE_MARKER)
}

fn has_true_impasse_rationale(text: &str) -> bool {
    has_any(text, USER_DECISION)
        && has_any(text, MATERIAL_CHANGE)
        && has_any(text, NO_SAFE_DEFAULT)
        && has_any(text, NO_IN_SCOPE_ACTION)
}
