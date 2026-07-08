use super::completion_handoff_pending_worktree_text::{
    char_window_start, has_any, has_false_value, has_nearby_negation, has_true_decision_value,
    phrase_has_boundaries,
};

const PENDING_WORKTREE_STATE_ERROR: &str = "pending worktree ids must resolve to a surfaced thread, explicit setup failure, or bounded timeout state with safe retry/reassignment evidence";

pub(super) fn check(text: &str) -> Option<String> {
    let pending_mentions = pending_worktree_mentions(text);
    if pending_mentions.is_empty() {
        return None;
    }
    for (index, start) in pending_mentions.iter().enumerate() {
        let end = pending_mentions
            .get(index + 1)
            .copied()
            .unwrap_or(text.len());
        let segment = &text[*start..end];
        if !mentions_surfaced_pending_worktree_thread(segment)
            && !mentions_failed_pending_worktree_setup(segment)
            && !mentions_bounded_pending_worktree_timeout(segment)
        {
            return Some(PENDING_WORKTREE_STATE_ERROR.into());
        }
    }
    None
}

fn pending_worktree_mentions(text: &str) -> Vec<usize> {
    let mut matches = Vec::new();
    for phrase in [
        "pendingworktreeid",
        "pending worktree id",
        "pending worktree ids",
        "pending worktree identifier",
        "pending worktree identifiers",
    ] {
        let mut rest = text;
        let mut offset = 0;
        while let Some(index) = rest.find(phrase) {
            let start = offset + index;
            let end = start + phrase.len();
            if phrase_has_boundaries(text, start, end)
                && !has_nearby_negation(&text[char_window_start(text, start, 16)..start])
                && !has_false_pending_value(&text[end..])
            {
                let id_starts = local_id_starts_before_outcome(text, end);
                if id_starts.is_empty() {
                    matches.push(start);
                } else {
                    matches.extend(id_starts);
                }
            }
            offset = end;
            rest = &text[offset..];
        }
    }
    matches.sort_unstable();
    matches.dedup();
    matches
}

fn local_id_starts_before_outcome(text: &str, start: usize) -> Vec<usize> {
    let suffix = &text[start..];
    let boundary = suffix
        .find('\n')
        .or_else(|| suffix.find('.'))
        .unwrap_or(suffix.len());
    let header = &suffix[..boundary];
    let mut starts = Vec::new();
    let mut rest = header;
    let mut offset = start;
    while let Some(index) = rest.find("local:") {
        let id_start = offset + index;
        starts.push(id_start);
        let value_end = local_id_value(text, id_start).map_or(id_start + "local:".len(), |value| {
            id_start + "local:".len() + value.len()
        });
        if text[value_end..].starts_with(':') {
            break;
        }
        offset = value_end;
        rest = &text[offset..start + boundary];
    }
    if starts.len() > 1 {
        if let Some(colon) = grouped_body_separator(text, start, &starts) {
            let body_start = colon + 1;
            let body = &text[body_start..start + boundary];
            let ordinal_starts: Vec<_> = starts
                .iter()
                .filter_map(|id_start| local_id_value(text, *id_start))
                .filter_map(|id| find_word(body, &id).map(|index| body_start + index))
                .collect();
            if ordinal_starts.len() == starts.len() {
                return ordinal_starts;
            }
        }
    }
    starts
}

fn grouped_body_separator(text: &str, sentence_start: usize, starts: &[usize]) -> Option<usize> {
    let last_start = *starts.last()?;
    let last_value = local_id_value(text, last_start)?;
    let after_last_value = last_start + "local:".len() + last_value.len();
    let sentence = &text[after_last_value..];
    sentence
        .find(':')
        .map(|index| after_last_value + index)
        .filter(|index| *index >= sentence_start)
}

fn local_id_value(text: &str, start: usize) -> Option<String> {
    let value = text.get(start + "local:".len()..)?;
    let value: String = value
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .collect();
    (!value.is_empty()).then_some(value)
}

fn find_word(text: &str, word: &str) -> Option<usize> {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(word) {
        let start = offset + index;
        let end = start + word.len();
        if phrase_has_boundaries(text, start, end) {
            return Some(start);
        }
        offset = end;
        rest = &text[offset..];
    }
    None
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
        && mentions_affirmative_safe_retry(text))
        || (has_any(
            text,
            "active lane accounting state is not-surfaced-after-bounded-wait",
        ) && mentions_bounded_search_evidence(text)
            && mentions_affirmative_safe_retry(text))
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

fn mentions_affirmative_safe_retry(text: &str) -> bool {
    has_any(
        text,
        "safe retry/reassignment is allowed|safe retry/reassignment allowed|safe retry is allowed|safe retry allowed|safe reassignment is allowed|safe reassignment allowed|retry/reassignment is allowed|retry/reassignment allowed",
    ) || has_true_decision_value(text, "safe retry/reassignment")
        || has_true_decision_value(text, "safe retry")
        || has_true_decision_value(text, "safe reassignment")
        || has_true_decision_value(text, "retry/reassignment")
}

fn has_false_pending_value(suffix: &str) -> bool {
    let value = suffix.trim_start();
    let Some(separator) = value.chars().next() else {
        return false;
    };
    if !matches!(separator, ':' | '=' | '-' | '?') {
        return false;
    }
    let value = value[separator.len_utf8()..].trim_start();
    has_false_value(value)
}
