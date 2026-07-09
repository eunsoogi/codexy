use super::completion_handoff_pending_worktree_text::has_any;

pub(super) fn colon_starts_lifecycle_entry(text: &str, colon: usize) -> bool {
    let suffix = &text[colon + 1..];
    let next_local = suffix.find("local:").unwrap_or(suffix.len());
    let entry = &suffix[..next_local];
    has_any(
        entry,
        "failed setup|setup failed|surfaced thread id|thread id|bounded timeout|bounded wait|remains unresolved|still unresolved|not visible|not surfaced",
    )
}

pub(super) fn bounded_search_evidence_text(text: &str) -> &str {
    text.split("metadata:").next().unwrap_or(text)
}

pub(super) fn has_quoted_terminal_false_value(value: &str) -> bool {
    let Some(value) = value.strip_prefix('"') else {
        return false;
    };
    "none|null|nil|false|no|n/a|n-a|na|not applicable|not-applicable|empty|missing|absent"
        .split('|')
        .any(|word| {
            value
                .strip_prefix(word)
                .is_some_and(is_terminal_json_decision_remainder)
        })
}

pub(super) fn pending_label_value_after_separator(suffix: &str) -> Option<&str> {
    let mut value = suffix.trim_start();
    if let Some(after_quote) = value.strip_prefix('"') {
        value = after_quote.trim_start();
    }
    let separator = value.chars().next()?;
    if !matches!(separator, ':' | '=' | '-' | '?') {
        return None;
    }
    Some(value[separator.len_utf8()..].trim_start())
}

pub(super) fn has_non_review_thread_id_evidence(text: &str) -> bool {
    let phrase = "thread id";
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if has_phrase_boundaries(text, start, end) && !is_review_thread_prefix(&text[..start]) {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn has_phrase_boundaries(text: &str, start: usize, end: usize) -> bool {
    text[..start]
        .chars()
        .next_back()
        .is_none_or(|character| !character.is_ascii_alphanumeric())
        && text[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn is_review_thread_prefix(prefix: &str) -> bool {
    prefix.ends_with("review ") || prefix.trim_end().ends_with("review-")
}

fn is_terminal_json_decision_remainder(remainder: &str) -> bool {
    let Some(remainder) = remainder.strip_prefix('"') else {
        return false;
    };
    let remainder = remainder.trim_start_matches([' ', '\t']);
    remainder.is_empty() || remainder.starts_with(['\n', '\r', ',', '.', ';', '}'])
}
