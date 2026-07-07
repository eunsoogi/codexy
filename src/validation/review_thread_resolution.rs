use serde_json::Value;

pub(super) use super::review_response_claim::claims_review_response;

const MISSING_REVIEW_THREADS: &str =
    "review response handoff missing reviewThreads.nodes PR state evidence";
const MISSING_PREVENTIVE_ADJACENT_REVIEW: &str = "review response handoff missing preventive adjacent review evidence: include focused preventive regression coverage for adjacent gaps in the touched helper family, or a concrete no-change rationale naming inspected functions/tests and why invariants hold";
pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    if !claims_review_response(handoff) {
        return Vec::new();
    }
    let Some(threads) = pr_state.get("reviewThreads") else {
        return vec![MISSING_REVIEW_THREADS.into()];
    };
    let Some(nodes) = threads.get("nodes").and_then(Value::as_array) else {
        return vec![MISSING_REVIEW_THREADS.into()];
    };
    if let Some(error) = super::review_thread_evidence::check(threads) {
        return vec![error];
    }
    let Some(unresolved) = nodes
        .iter()
        .filter(|thread| thread.get("isResolved").and_then(Value::as_bool) == Some(false))
        .find(|thread| {
            !documents_accepted_no_change_rationale(handoff, thread)
                && !super::review_thread_waiting::documents_unfixed_or_unaccepted(handoff, thread)
        })
    else {
        if super::preventive_adjacent_review::documents_incomplete_or_blocked_state(handoff)
            || super::preventive_adjacent_review::documents_preventive_adjacent_review(handoff)
        {
            return Vec::new();
        }
        return vec![MISSING_PREVENTIVE_ADJACENT_REVIEW.into()];
    };
    vec![format!(
        "unresolved review thread remains after addressed review feedback: {}; resolve fixed threads after current-head verification or document an accepted no-change rationale",
        thread_label(unresolved)
    )]
}
pub(super) fn documents_accepted_no_change_rationale(handoff: &str, thread: &Value) -> bool {
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
fn local_action_prefix(prefix: &str) -> &str {
    let p = prefix.rfind(['\n', ',', ';']).map(|index| index + 1);
    let s = prefix.rfind(". ").map(|index| index + 1);
    let c = prefix.rfind(" but ").map(|index| index + 5);
    let start = [p, s, c].into_iter().flatten().max().unwrap_or(0);
    &prefix[start..]
}
fn has_post_label_negation(segment: &str, phrase: &str) -> bool {
    let after_label = segment
        .strip_prefix(phrase)
        .unwrap_or_default()
        .trim_start_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ':' | '-'));
    if ["is missing", "missing"]
        .iter()
        .any(|word| has_word_prefix(after_label, word))
    {
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
