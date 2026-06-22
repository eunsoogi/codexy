use serde_json::Value;

pub(super) fn documents_unfixed_or_unaccepted(handoff: &str, thread: &Value) -> bool {
    if claims_readiness(handoff) || claims_completion(handoff) {
        return false;
    }
    let text = handoff.to_ascii_lowercase();
    waiting_segments(&text).any(|segment| {
        thread_referenced(segment, thread)
            && mentions_unresolved(segment)
            && (mentions_not_fixed(segment) || mentions_not_accepted(segment))
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
    text.as_bytes().get(dot_index) == Some(&b'.') && !url_token_before_dot(&text[..dot_index])
}

fn url_token_before_dot(prefix: &str) -> bool {
    let start = prefix
        .rfind(|character: char| {
            character.is_ascii_whitespace() || matches!(character, '<' | '(' | '[')
        })
        .map_or(0, |index| index + 1);
    let token = &prefix[start..];
    token.starts_with("http://") || token.starts_with("https://")
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
    .any(|phrase| has_unnegated_phrase(&text, phrase))
}

fn claims_completion(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
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

fn has_unnegated_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(start, _)| {
        let prefix_start = char_window_start(text, start, 16);
        let prefix = &text[prefix_start..start];
        is_boundary(text[..start].chars().next_back())
            && is_boundary(text[start + phrase.len()..].chars().next())
            && !has_nearby_negation(prefix)
    })
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
        "unfixed",
        "not addressed",
        "not yet addressed",
        "not fixed/accepted",
        "not fixed or accepted",
    ]
    .iter()
    .any(|term| segment.contains(term))
}

fn mentions_not_accepted(segment: &str) -> bool {
    [
        "not accepted",
        "not yet accepted",
        "not fixed/accepted",
        "not fixed or accepted",
    ]
    .iter()
    .any(|term| segment.contains(term))
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

fn has_nearby_negation(prefix: &str) -> bool {
    ["no", "not", "not yet", "without", "neither"]
        .iter()
        .any(|term| prefix.trim_end().ends_with(term))
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}
