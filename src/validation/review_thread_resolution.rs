use serde_json::Value;
const MISSING_REVIEW_THREADS: &str =
    "review response handoff missing reviewThreads.nodes PR state evidence";
pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    if !claims_review_response(handoff) {
        return Vec::new();
    }
    let Some(threads) = pr_state.get("reviewThreads") else {
        return vec![MISSING_REVIEW_THREADS.into()];
    };
    let Some(nodes) = review_thread_nodes(threads) else {
        return vec![MISSING_REVIEW_THREADS.into()];
    };
    if let Some(error) = super::review_thread_evidence::check(threads) {
        return vec![error];
    }
    let Some(unresolved) = nodes
        .iter()
        .filter(|thread| thread.get("isResolved").and_then(Value::as_bool) == Some(false))
        .find(|thread| !documents_accepted_no_change_rationale(handoff, thread))
    else {
        return Vec::new();
    };
    vec![format!(
        "unresolved review thread remains after addressed review feedback: {}; resolve fixed threads after current-head verification or document an accepted no-change rationale",
        thread_label(unresolved)
    )]
}
fn review_thread_nodes(threads: &Value) -> Option<&Vec<Value>> {
    threads.get("nodes").and_then(Value::as_array)
}
fn claims_review_response(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    has_any(
        &text,
        &[
            "accepted no-change rationale",
            "accepted no change rationale",
            "no-change rationale documented",
            "no change rationale documented",
        ],
    ) || review_feedback_segments(&text).any(|segment| {
        "addressed applied fixed fixes handled implemented responded resolved resolve resolves updated"
            .split_whitespace()
            .any(|phrase| has_unnegated_action(segment, phrase))
    })
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
fn review_feedback_segments(text: &str) -> impl Iterator<Item = &str> {
    let mut section = false;
    text.split_inclusive(['.', '\n', ';'])
        .filter(move |segment| {
            let has_context =
                "codex review|codex feedback|review response|review feedback|review thread|review comment|review comments|review suggestion|review suggestions"
                    .split('|')
                    .any(|term| segment.contains(term));
            let trimmed = segment.trim_start();
            let matches = has_context || (section && trimmed.starts_with('-'));
            section = (has_context && segment.trim_end().ends_with(':'))
                || (section && (trimmed.starts_with('-') || trimmed.is_empty()));
            matches
        })
}
fn has_unnegated_action(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if !is_word_match(text, start, end) {
            offset = end;
            rest = &text[offset..];
            continue;
        }
        let prefix = &text[..start];
        let local_prefix = local_action_prefix(prefix);
        let clean_review_prefix = prefix.trim_end().ends_with("codex review passed,")
            && !text[start..].contains("review");
        if !clean_review_prefix
            && !has_any(
                prefix,
                &[
                    "review response: none",
                    "review feedback: none",
                    "review thread: none",
                    "review comments: none",
                    "none from codex",
                ],
            )
            && !"no review feedback was |no review feedback |no feedback was |no feedback |not "
                .split('|')
                .any(|negation| local_prefix.contains(negation))
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}
fn is_word_match(text: &str, start: usize, end: usize) -> bool {
    let b = text.as_bytes();
    !b.get(start.wrapping_sub(1))
        .is_some_and(u8::is_ascii_alphanumeric)
        && !b.get(end).is_some_and(u8::is_ascii_alphanumeric)
}
fn local_action_prefix(prefix: &str) -> &str {
    let start = prefix
        .rfind(['\n', ',', ';'])
        .map(|index| index + 1)
        .into_iter()
        .chain(prefix.rfind(". ").map(|index| index + 1))
        .chain(prefix.rfind(" but ").map(|index| index + 5))
        .max()
        .unwrap_or(0);
    &prefix[start..]
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
    let url = comment_urls(thread).next().unwrap_or("no comment URL");
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
            if !is_negated_rationale(text, start, phrase, segment) && !is_empty_rationale(segment) {
                return Some(segment);
            }
        }
    })
}
fn is_negated_rationale(text: &str, start: usize, phrase: &str, segment: &str) -> bool {
    let prefix = local_action_prefix(&text[..start]);
    ["no ", "not ", "without ", "missing "]
        .iter()
        .any(|negation| prefix.contains(negation))
        || has_post_label_negation(segment, phrase)
}
fn has_post_label_negation(segment: &str, phrase: &str) -> bool {
    let after_label = segment
        .strip_prefix(phrase)
        .unwrap_or_default()
        .trim_start_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ':' | '-'));
    if has_word_prefix(after_label, "is missing") {
        return true;
    }
    let prefixes = "not |was not |wasn't |is not |isn't |has not been |hasn't been ";
    let words = ["accepted", "approved", "documented"];
    prefixes
        .split('|')
        .filter_map(|prefix| after_label.strip_prefix(prefix))
        .any(|rest| words.iter().any(|word| has_word_prefix(rest, word)))
}
fn has_word_prefix(text: &str, word: &str) -> bool {
    text.strip_prefix(word).is_some_and(|tail| {
        tail.as_bytes()
            .first()
            .is_none_or(|byte| !byte.is_ascii_alphanumeric())
    })
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
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|ch| !is_reference_char(ch)) && after.is_none_or(|ch| !is_reference_char(ch))
}
fn is_reference_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '#' | ':')
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
