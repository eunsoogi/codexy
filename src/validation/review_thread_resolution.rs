use serde_json::Value;

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    if !claims_review_response(handoff) {
        return Vec::new();
    }
    let Some(nodes) = review_thread_nodes(pr_state) else {
        return vec![
            "review response handoff missing reviewThreads.nodes PR state evidence".into(),
        ];
    };
    if let Some(error) = super::review_thread_evidence::check_nodes(nodes) {
        return vec![error];
    }
    let unresolved = nodes
        .iter()
        .filter(|thread| is_unresolved_thread(thread))
        .find(|thread| !documents_accepted_no_change_rationale(handoff, thread));
    if unresolved.is_none() {
        return Vec::new();
    }
    vec![format!(
        "unresolved review thread remains after addressed review feedback: {}; resolve fixed threads after current-head verification or document an accepted no-change rationale",
        thread_label(unresolved.expect("checked unresolved thread"))
    )]
}

fn review_thread_nodes(pr_state: &Value) -> Option<&Vec<Value>> {
    pr_state
        .get("reviewThreads")
        .and_then(|threads| threads.get("nodes"))
        .and_then(Value::as_array)
}

fn is_unresolved_thread(thread: &Value) -> bool {
    thread
        .get("isResolved")
        .and_then(Value::as_bool)
        .is_some_and(|resolved| !resolved)
}

fn claims_review_response(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    let has_review_feedback = has_any(
        &text,
        &["review response", "review feedback", "review thread"],
    );
    has_any(
        &text,
        &[
            "accepted no-change rationale",
            "accepted no change rationale",
            "no-change rationale documented",
            "no change rationale documented",
        ],
    ) || (has_review_feedback
        && ["addressed", "fixed", "responded"]
            .iter()
            .any(|phrase| has_unnegated_action(&text, phrase)))
}

fn documents_accepted_no_change_rationale(handoff: &str, thread: &Value) -> bool {
    let text = handoff.to_ascii_lowercase();
    [
        "accepted no-change rationale",
        "accepted no change rationale",
        "no-change rationale documented",
        "no change rationale documented",
    ]
    .iter()
    .any(|phrase| {
        rationale_segments(&text, phrase).any(|segment| thread_referenced(segment, thread))
    })
}

fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn has_unnegated_action(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let prefix_start = char_window_start(text, start, 32);
        let prefix = &text[prefix_start..start];
        if ![
            "no review feedback was ",
            "no review feedback ",
            "no feedback was ",
            "no feedback ",
            "not ",
        ]
        .iter()
        .any(|negation| prefix.contains(negation))
        {
            return true;
        }
        offset = start + phrase.len();
        rest = &text[offset..];
    }
    false
}

fn char_window_start(text: &str, end: usize, max_chars: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(max_chars.saturating_sub(1))
        .map_or(0, |(index, _)| index)
}

fn thread_label(thread: &Value) -> String {
    let id = thread
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown thread");
    let path = thread
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("unknown path");
    let url = first_comment_url(thread).unwrap_or("no comment URL");
    format!("{id} at {path} ({url})")
}

fn thread_referenced(text: &str, thread: &Value) -> bool {
    thread
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| has_exact_reference(text, &id.to_ascii_lowercase()))
        || comment_urls(thread).any(|url| has_exact_reference(text, &url.to_ascii_lowercase()))
}

fn rationale_segments<'a>(text: &'a str, phrase: &str) -> impl Iterator<Item = &'a str> {
    let mut rest = text;
    let mut offset = 0;
    std::iter::from_fn(move || {
        loop {
            let index = rest.find(phrase)?;
            let start = offset + index;
            let end = clause_end(text, start);
            offset = start + phrase.len();
            rest = &text[offset..];
            let segment = &text[start..end];
            if !is_negated_rationale(text, start) && !is_empty_rationale(segment) {
                return Some(segment);
            }
        }
    })
}

fn is_negated_rationale(text: &str, start: usize) -> bool {
    let prefix_start = char_window_start(text, start, 32);
    let prefix = &text[prefix_start..start];
    ["no ", "not ", "without ", "missing "]
        .iter()
        .any(|negation| prefix.contains(negation))
}

fn is_empty_rationale(segment: &str) -> bool {
    [": none", ": n/a", ": not applicable", "- none", "- n/a"]
        .iter()
        .any(|empty| segment.contains(empty))
}

fn clause_end(text: &str, start: usize) -> usize {
    let suffix = &text[start..];
    [
        suffix.find('\n'),
        suffix.find(". "),
        suffix.find(','),
        suffix.find(';'),
    ]
    .into_iter()
    .flatten()
    .min()
    .map_or(text.len(), |index| start + index)
}

fn has_exact_reference(text: &str, reference: &str) -> bool {
    if reference.is_empty() {
        return false;
    }
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(reference) {
        let start = offset + index;
        let end = start + reference.len();
        if is_reference_boundary(text, start, end) {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn is_reference_boundary(text: &str, start: usize, end: usize) -> bool {
    text[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_reference_char(ch))
        && text[end..]
            .chars()
            .next()
            .is_none_or(|ch| !is_reference_char(ch))
}

fn is_reference_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '#' | ':')
}

fn first_comment_url(thread: &Value) -> Option<&str> {
    comment_urls(thread).next()
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
