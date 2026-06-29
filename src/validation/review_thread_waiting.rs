use serde_json::Value;

use super::review_thread_waiting_phrases::{
    has_unnegated_action_phrase, has_unnegated_phrase, has_unnegated_readiness_phrase,
};
use super::review_thread_waiting_refs::{
    first_review_reference_start, thread_referenced, thread_waiting_clauses,
};

pub(super) fn documents_unfixed_or_unaccepted(handoff: &str, thread: &Value) -> bool {
    let text = handoff.to_ascii_lowercase();
    if claims_readiness(handoff) || claims_completion(handoff) || claims_thread_fixed(&text, thread)
    {
        return false;
    }
    waiting_evidence_segments(&text, thread)
        .iter()
        .any(|segment| {
            thread_waiting_clauses(segment, thread)
                .iter()
                .any(|segment| {
                    mentions_unresolved(segment)
                        && mentions_not_fixed(segment)
                        && mentions_not_accepted(segment)
                })
        })
}

fn waiting_evidence_segments(text: &str, thread: &Value) -> Vec<String> {
    let mut segments = Vec::new();
    let mut carry = String::new();
    for segment in waiting_segments(text) {
        if !carry.is_empty() && !thread_referenced(segment, thread) {
            if let Some(reference_start) = first_review_reference_start(segment) {
                let (prefix, suffix) = segment.split_at(reference_start);
                if prefix.trim().is_empty() {
                    segments.push(std::mem::take(&mut carry));
                } else {
                    segments.push(format!("{carry}{prefix}"));
                    carry.clear();
                }
                segments.push(suffix.to_string());
                continue;
            }
        }
        let candidate = format!("{carry}{segment}");
        let continues_waiting_clause = (segment.ends_with(';')
            || (segment.ends_with('\n') && segment.trim_end().ends_with(':'))
            || (!carry.is_empty() && is_markdown_list_item(segment)))
            && thread_referenced(&candidate, thread)
            && mentions_unresolved(&candidate);
        carry.push_str(segment);
        if continues_waiting_clause {
            continue;
        }
        segments.push(std::mem::take(&mut carry));
    }
    if !carry.is_empty() {
        segments.push(carry);
    }
    segments
}

fn is_markdown_list_item(segment: &str) -> bool {
    segment.trim_start().starts_with("- ")
}

fn waiting_segments(text: &str) -> impl Iterator<Item = &str> {
    let mut start = 0;
    std::iter::from_fn(move || {
        if start >= text.len() {
            return None;
        }
        let suffix = &text[start..];
        for (relative_index, character) in suffix.char_indices() {
            if character == '\n' || character == ';' || splits_sentence_dot(suffix, relative_index)
            {
                let end = start + relative_index + character.len_utf8();
                let segment = &text[start..end];
                start = end;
                return Some(segment);
            }
        }
        let segment = &text[start..];
        start = text.len();
        Some(segment)
    })
}

fn splits_sentence_dot(text: &str, dot_index: usize) -> bool {
    text.as_bytes().get(dot_index) == Some(&b'.')
        && !dot_inside_url_token(text, dot_index)
        && !dot_inside_path_token(text, dot_index)
}

fn dot_inside_url_token(text: &str, dot_index: usize) -> bool {
    let prefix = &text[..dot_index];
    let start = prefix
        .rfind(|character: char| {
            character.is_ascii_whitespace() || matches!(character, '<' | '(' | '[')
        })
        .map_or(0, |index| index + 1);
    let token = &prefix[start..];
    (token.starts_with("http://") || token.starts_with("https://"))
        && text[dot_index + 1..]
            .chars()
            .next()
            .is_some_and(is_reference_char)
}

fn dot_inside_path_token(text: &str, dot_index: usize) -> bool {
    let prefix = &text[..dot_index];
    let start = prefix
        .rfind(|character: char| character.is_ascii_whitespace())
        .map_or(0, |index| index + 1);
    let previous_is_token_char = prefix[start..]
        .chars()
        .next_back()
        .is_some_and(is_reference_char);
    let next_is_token_char = text[dot_index + 1..]
        .chars()
        .next()
        .is_some_and(is_reference_char);
    next_is_token_char && (previous_is_token_char || prefix[start..].contains('/'))
}

fn claims_readiness(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    "pr ready|pr-ready|pr is ready|pull request ready|pull-request-ready|pull request is ready|pr readiness|pr-readiness|ready for parent handoff|ready for handoff|ready for merge|ready to merge|merge readiness|merge-readiness|merge ready|merge-ready|codex review passed|codex review completed|codex review complete|codex review approved"
        .split('|')
        .any(|phrase| has_unnegated_readiness_phrase(&text, phrase))
}

fn claims_completion(handoff: &str) -> bool {
    let mut text = handoff.to_ascii_lowercase();
    if has_not_complete_until_merge(&text) {
        text = text.replace("verification completed.", "verification evidence.");
        text = text.replace("verification completed:", "verification evidence:");
        for phrase in [
            "successfully completed",
            "completed successfully",
            "completed",
            "finished",
            "finalized",
        ] {
            text = text.replace(&format!("verification {phrase};"), "verification evidence;");
        }
    }
    [
        "completed",
        "finished",
        "finalized",
        "all set",
        "done",
        "complete",
        "completes",
        "finish",
        "finalize",
    ]
    .iter()
    .any(|phrase| has_unnegated_phrase(&text, phrase))
}

fn has_not_complete_until_merge(text: &str) -> bool {
    "not complete until merge|not currently complete until merge"
        .split('|')
        .any(|phrase| has_unnegated_phrase(text, phrase))
}

fn mentions_unresolved(segment: &str) -> bool {
    ["unresolved", "still open", "remains open", "left open"]
        .iter()
        .any(|term| segment.contains(term))
}

fn mentions_not_fixed(segment: &str) -> bool {
    "not fixed|not yet fixed|isn't fixed|isn't yet fixed|wasn't fixed|has not been fixed|has not yet been fixed|hasn't been fixed|hasn't yet been fixed|unfixed|not addressed|not yet addressed|isn't addressed|wasn't addressed|has not been addressed|has not yet been addressed|hasn't been addressed|hasn't yet been addressed|not fixed/accepted|not fixed or accepted|isn't fixed/accepted|isn't fixed or accepted"
        .split('|')
    .any(|term| segment.contains(term))
}

fn mentions_not_accepted(segment: &str) -> bool {
    "not accepted|not yet accepted|isn't accepted|isn't yet accepted|wasn't accepted|has not been accepted|has not yet been accepted|hasn't been accepted|hasn't yet been accepted|not fixed/accepted|not fixed or accepted|not yet fixed/accepted|not yet fixed or accepted|has not been fixed or accepted|has not yet been fixed or accepted|hasn't been fixed or accepted|hasn't yet been fixed or accepted|isn't fixed/accepted|isn't fixed or accepted|isn't yet fixed/accepted|isn't yet fixed or accepted"
        .split('|')
    .any(|term| segment.contains(term))
}

fn claims_thread_fixed(text: &str, thread: &Value) -> bool {
    waiting_segments(text).any(|segment| {
        action_claim_segments(segment).any(|claim| {
            thread_referenced(claim, thread)
                && "accepted accepts addressed addresses addressing applied fixed fixes handled implemented responded resolved resolve resolves updated"
                    .split_whitespace()
                    .any(|action| {
                        has_unnegated_action_phrase(claim, action)
                            && !(matches!(action, "accepted" | "accepts")
                                && mentions_not_accepted(claim))
                    })
        })
    })
}

fn action_claim_segments(segment: &str) -> impl Iterator<Item = &str> {
    segment
        .split(',')
        .flat_map(|clause| clause.split(" but "))
        .flat_map(|clause| clause.split(" and thread "))
        .flat_map(split_and_url_reference)
        .flat_map(|clause| clause.split(" and it "))
        .flat_map(|clause| clause.split(": remains unresolved"))
        .flat_map(|clause| clause.split(" and remains unresolved"))
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
}

fn split_and_url_reference(clause: &str) -> Vec<&str> {
    let split_at = [" and https://", " and http://"]
        .iter()
        .filter_map(|marker| clause.find(marker))
        .filter(|&index| {
            let suffix = &clause[index + 5..];
            let unresolved = ["unresolved", "still open", "remains open", "left open"]
                .iter()
                .filter_map(|term| suffix.find(term))
                .min();
            let next_url = [" and https://", " and http://"]
                .iter()
                .filter_map(|marker| suffix.find(marker))
                .min();
            unresolved.is_some_and(|unresolved| next_url.is_none_or(|next| unresolved < next))
        })
        .min();
    split_at.map_or_else(
        || vec![clause],
        |index| vec![&clause[..index], &clause[index + 5..]],
    )
}

fn is_reference_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '#' | ':')
}
