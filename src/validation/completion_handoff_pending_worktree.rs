use super::completion_handoff_pending_worktree_labels::has_false_actionable_error_evidence;
use super::completion_handoff_pending_worktree_segments::{
    bare_pending_mention_has_state, bounded_search_evidence_text, colon_starts_lifecycle_entry,
    has_non_review_thread_id_evidence, has_quoted_terminal_false_value,
    pending_label_value_after_separator,
};
use super::completion_handoff_pending_worktree_text::{
    char_window_start, find_word, has_any, has_false_bounded_search_evidence,
    has_false_surfaced_thread_evidence, has_nearby_negation, has_negated_pending_return,
    has_terminal_false_value, has_true_decision_value, is_markdown_list_item, local_id_value,
    ordinal_label, phrase_has_boundaries,
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
        "pendingworktreeids",
        "pending worktree ids",
        "pendingworktreeid",
        "pending worktree id",
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
                && !has_negated_pending_return(text, start, end)
                && !has_false_pending_value(&text[end..])
            {
                let id_starts = local_id_starts_before_outcome(text, end);
                if id_starts.is_empty() {
                    if bare_pending_mention_has_state(&text[end..]) {
                        matches.push(start);
                    }
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
    let mut seen_ids = Vec::new();
    matches.retain(|start| {
        local_id_value(text, *start).is_none_or(|id| {
            !seen_ids.contains(&id) && {
                seen_ids.push(id);
                true
            }
        })
    });
    matches
}
fn local_id_starts_before_outcome(text: &str, start: usize) -> Vec<usize> {
    let suffix = &text[start..];
    let boundary = suffix.find(['\n', '.']).unwrap_or(suffix.len());
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
        if text[value_end..].starts_with(':') && !colon_starts_lifecycle_entry(text, value_end) {
            break;
        }
        offset = value_end;
        rest = &text[offset..start + boundary];
    }
    if starts.is_empty() {
        return local_id_starts_in_following_list(text, start + boundary);
    }
    if starts.len() > 1 {
        if let Some(colon) = grouped_body_separator(text, start, start + boundary, &starts) {
            let body_start = colon + 1;
            let body = &text[body_start..start + boundary];
            let ordinal_starts: Vec<_> = starts
                .iter()
                .enumerate()
                .filter_map(|(index, id_start)| {
                    let id = local_id_value(text, *id_start)?;
                    find_word(body, &id)
                        .or_else(|| ordinal_label(index).and_then(|word| find_word(body, word)))
                        .map(|body_index| body_start + body_index)
                })
                .collect();
            if ordinal_starts.len() == starts.len() {
                return ordinal_starts;
            }
        }
    }
    starts
}
fn local_id_starts_in_following_list(text: &str, list_header_end: usize) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut offset = list_header_end;
    let mut rest = &text[list_header_end..];
    if rest.starts_with('\n') {
        offset += 1;
        rest = &text[offset..];
    } else {
        return starts;
    }
    for line in rest.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches('\n');
        let trimmed = line_without_newline.trim_start();
        if !is_markdown_list_item(trimmed) {
            break;
        }
        let line_indent = line_without_newline.len() - trimmed.len();
        let line_start = offset + line_indent;
        let mut item_rest = trimmed;
        let mut item_offset = line_start;
        while let Some(index) = item_rest.find("local:") {
            let id_start = item_offset + index;
            starts.push(id_start);
            let value_end = local_id_value(text, id_start)
                .map_or(id_start + "local:".len(), |value| {
                    id_start + "local:".len() + value.len()
                });
            item_offset = value_end;
            item_rest = &text[item_offset..offset + line_without_newline.len()];
        }
        offset += line.len();
    }
    starts
}

fn grouped_body_separator(
    text: &str,
    sentence_start: usize,
    sentence_end: usize,
    starts: &[usize],
) -> Option<usize> {
    let last_start = *starts.last()?;
    let last_value = local_id_value(text, last_start)?;
    let after_last_value = last_start + "local:".len() + last_value.len();
    if after_last_value >= sentence_end {
        return None;
    }
    let sentence = &text[after_last_value..sentence_end];
    sentence
        .find(':')
        .map(|index| after_last_value + index)
        .filter(|index| *index >= sentence_start)
}
fn mentions_surfaced_pending_worktree_thread(text: &str) -> bool {
    !has_false_surfaced_thread_evidence(text)
        && (has_any(
            text,
            "surfaced thread id|observed thread id|resolved to thread",
        ) || has_non_review_thread_id_evidence(text))
        && has_any(text, "active owner|active lane accounting state is active")
}

fn mentions_failed_pending_worktree_setup(text: &str) -> bool {
    !has_false_actionable_error_evidence(text)
        && has_any(
            text,
            "failed setup|setup failed|explicit failed setup state|active lane accounting state is failed",
        )
        && mentions_actionable_setup_failure_detail(text)
}

fn mentions_actionable_setup_failure_detail(text: &str) -> bool {
    has_any(
        text,
        "actionable error|fatal|invalid reference|does not exist|corrected base ref",
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
    let evidence = bounded_search_evidence_text(text);
    !has_false_bounded_search_evidence(evidence)
        && has_any(
            evidence,
            "searches by pending id|searches by pending worktree id|searched by pending id|searched by pending worktree id|list_threads searches by pending id|list_threads searches by pending worktree id",
        )
        && has_any(evidence, "branch")
        && has_any(evidence, "pr|pull request|issue")
        && has_any(evidence, "sha|commit")
        && has_any(
            evidence,
            "review-thread id|review thread id|available review-thread id|available review thread id|no review-thread id available|no review thread id available",
        )
}

fn mentions_affirmative_safe_retry(text: &str) -> bool {
    has_true_decision_value(text, "safe retry/reassignment")
        || has_true_decision_value(text, "safe retry or reassignment")
        || has_true_decision_value(text, "safe retry")
        || has_true_decision_value(text, "safe reassignment")
        || has_true_decision_value(text, "retry/reassignment")
        || has_true_decision_value(text, "retry or reassignment")
}

fn has_false_pending_value(suffix: &str) -> bool {
    pending_label_value_after_separator(suffix).is_some_and(|value| {
        has_terminal_false_value(value) || has_quoted_terminal_false_value(value)
    })
}
