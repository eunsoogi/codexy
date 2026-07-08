const PENDING_WORKTREE_STATE_ERROR: &str = "pending worktree ids must resolve to a surfaced thread, explicit setup failure, or bounded timeout state with safe retry/reassignment evidence";

pub(super) fn check(text: &str) -> Option<String> {
    if mentions_pending_worktree_id(text)
        && !mentions_surfaced_pending_worktree_thread(text)
        && !mentions_failed_pending_worktree_setup(text)
        && !mentions_bounded_pending_worktree_timeout(text)
    {
        return Some(PENDING_WORKTREE_STATE_ERROR.into());
    }
    None
}

fn mentions_pending_worktree_id(text: &str) -> bool {
    has_any(
        text,
        "pendingworktreeid|pending worktree id|pending worktree ids|pending worktree identifier|pending worktree identifiers",
    )
}

fn mentions_surfaced_pending_worktree_thread(text: &str) -> bool {
    has_any(
        text,
        "surfaced thread id|observed thread id|resolved to thread|thread id",
    ) && has_any(text, "active owner|active lane accounting state is active")
}

fn mentions_failed_pending_worktree_setup(text: &str) -> bool {
    has_any(
        text,
        "failed setup|setup failed|explicit failed setup state|active lane accounting state is failed",
    ) && has_any(
        text,
        "actionable error|fatal|invalid reference|does not exist|missing|corrected base ref",
    )
}

fn mentions_bounded_pending_worktree_timeout(text: &str) -> bool {
    (has_any(
        text,
        "bounded timeout|bounded wait|not-surfaced-after-bounded-wait|not surfaced after bounded wait|not surfaced after a bounded wait",
    ) && mentions_bounded_search_evidence(text)
        && has_any(
            text,
            "safe retry|safe reassignment|safe retry/reassignment|retry/reassignment",
        ))
        || (has_any(
            text,
            "active lane accounting state is not-surfaced-after-bounded-wait",
        ) && mentions_bounded_search_evidence(text)
            && has_any(
                text,
                "safe retry|safe reassignment|safe retry/reassignment|retry/reassignment",
            ))
}

fn mentions_bounded_search_evidence(text: &str) -> bool {
    has_any(
        text,
        "searches by pending id|searches by pending worktree id|searched by pending id|searched by pending worktree id|list_threads searches by pending id|list_threads searches by pending worktree id",
    ) && has_any(text, "branch")
        && has_any(text, "pr|pull request|issue")
        && has_any(text, "sha|commit")
        && has_any(
            text,
            "review-thread id|review thread id|available review-thread id|available review thread id|no review-thread id available|no review thread id available",
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

fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

fn is_boundary(c: Option<char>) -> bool {
    c.is_none_or(|c| !c.is_ascii_alphanumeric())
}

fn has_nearby_negation(prefix: &str) -> bool {
    "no|no known|no longer|non|non-|not|not a|not an|isn't|is not|hasn't|without"
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
