use serde_json::Value;

use super::review_thread_waiting_phrases::{
    has_unnegated_action_phrase, has_unnegated_phrase, has_unnegated_readiness_phrase,
};

pub(super) fn documents_unfixed_or_unaccepted(handoff: &str, thread: &Value) -> bool {
    let text = handoff.to_ascii_lowercase();
    if claims_readiness(handoff) || claims_completion(handoff) || claims_thread_fixed(&text, thread)
    {
        return false;
    }
    waiting_segments(&text).any(|segment| {
        thread_referenced(segment, thread)
            && mentions_unresolved(segment)
            && mentions_not_fixed(segment)
            && mentions_not_accepted(segment)
    })
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
    [
        "pr ready",
        "pr readiness",
        "pr-readiness",
        "ready for parent handoff",
        "ready for handoff",
        "ready for merge",
        "ready to merge",
        "merge ready",
        "merge-ready",
    ]
    .iter()
    .any(|phrase| has_unnegated_readiness_phrase(&text, phrase))
}

fn claims_completion(handoff: &str) -> bool {
    let mut text = handoff.to_ascii_lowercase();
    if has_unnegated_phrase(&text, "not complete until merge") {
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

fn mentions_unresolved(segment: &str) -> bool {
    ["unresolved", "still open", "remains open", "left open"]
        .iter()
        .any(|term| segment.contains(term))
}

fn mentions_not_fixed(segment: &str) -> bool {
    [
        "not fixed",
        "not yet fixed",
        "isn't fixed",
        "isn't yet fixed",
        "wasn't fixed",
        "has not yet been fixed",
        "hasn't been fixed",
        "hasn't yet been fixed",
        "unfixed",
        "not addressed",
        "not yet addressed",
        "isn't addressed",
        "wasn't addressed",
        "has not yet been addressed",
        "hasn't been addressed",
        "hasn't yet been addressed",
        "not fixed/accepted",
        "not fixed or accepted",
        "isn't fixed/accepted",
        "isn't fixed or accepted",
    ]
    .iter()
    .any(|term| segment.contains(term))
}

fn mentions_not_accepted(segment: &str) -> bool {
    "not accepted|not yet accepted|isn't accepted|isn't yet accepted|wasn't accepted|has not yet been accepted|hasn't been accepted|hasn't yet been accepted|not fixed/accepted|not fixed or accepted|not yet fixed/accepted|not yet fixed or accepted|isn't fixed/accepted|isn't fixed or accepted|isn't yet fixed/accepted|isn't yet fixed or accepted"
        .split('|')
    .any(|term| segment.contains(term))
}

fn claims_thread_fixed(text: &str, thread: &Value) -> bool {
    waiting_segments(text).any(|segment| {
        thread_referenced(segment, thread)
            && "addressed addresses addressing applied fixed fixes handled implemented responded resolved resolve resolves updated"
                .split_whitespace()
                .any(|action| has_unnegated_action_phrase(segment, action))
    })
}

fn thread_referenced(text: &str, thread: &Value) -> bool {
    thread
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| has_exact_reference(text, &id.to_ascii_lowercase()))
        || comment_urls(thread).any(|url| has_exact_reference(text, &url.to_ascii_lowercase()))
}

fn comment_urls(thread: &Value) -> impl Iterator<Item = &str> {
    thread
        .get("comments")
        .and_then(|comments| comments.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|nodes| nodes.iter())
        .filter_map(|comment| comment.get("url").and_then(Value::as_str))
}

fn has_exact_reference(text: &str, reference: &str) -> bool {
    !reference.is_empty()
        && text.match_indices(reference).any(|(start, _)| {
            let end = start + reference.len();
            let before = text[..start].chars().next_back();
            let after = text[end..].chars().next();
            before.is_none_or(|ch| !is_reference_char(ch))
                && after.is_none_or(|ch| !is_reference_char(ch))
        })
}

fn is_reference_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '#' | ':')
}
