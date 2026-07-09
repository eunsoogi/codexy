use super::completion_handoff_pending_worktree_labels::has_false_label_value;

pub(super) fn has_any(text: &str, phrases: &str) -> bool {
    phrases
        .split('|')
        .any(|phrase| has_unnegated_phrase(text, phrase, 16))
}

pub(super) fn has_false_value(value: &str) -> bool {
    "none|null|nil|false|no|n/a|n-a|na|not applicable|not-applicable|empty|missing|absent"
        .split('|')
        .any(|word| value.strip_prefix(word).is_some_and(starts_with_boundary))
}

pub(super) fn has_terminal_false_value(value: &str) -> bool {
    "none|null|nil|false|no|n/a|n-a|na|not applicable|not-applicable|empty|missing|absent"
        .split('|')
        .any(|word| {
            value
                .strip_prefix(word)
                .is_some_and(is_terminal_decision_remainder)
        })
}

pub(super) fn has_true_decision_value(text: &str, label: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(label) {
        let start = offset + index;
        let end = start + label.len();
        if phrase_has_boundaries(text, start, end)
            && !has_nearby_negation(&text[char_window_start(text, start, 16)..start])
        {
            let suffix = text[end..].trim_start();
            if suffix.starts_with("is not allowed")
                || suffix.starts_with("not allowed")
                || suffix.starts_with("is unsafe")
                || suffix.starts_with("unsafe")
            {
                offset = end;
                rest = &text[offset..];
                continue;
            }
            for value in ["is allowed", "allowed", "is safe", "safe"] {
                if let Some(remainder) = suffix
                    .strip_prefix(value)
                    .filter(|remainder| starts_with_boundary(remainder))
                {
                    if has_explicit_false_value(remainder) {
                        continue;
                    }
                    if has_unsafe_decision_remainder(remainder) {
                        continue;
                    }
                    return true;
                }
            }
            if let Some(value) = suffix.strip_prefix([':', '=', '-', '?']) {
                let value = value.trim_start();
                if has_true_value(value) && !has_unsafe_decision_remainder(value) {
                    return true;
                }
            }
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

pub(super) fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

pub(super) fn is_markdown_list_item(line: &str) -> bool {
    if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
        return true;
    }
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    digits > 0 && line[digits..].starts_with(". ")
}

pub(super) fn ordinal_label(index: usize) -> Option<&'static str> {
    ["first", "second", "third", "fourth", "fifth"]
        .get(index)
        .copied()
}

pub(super) fn local_id_value(text: &str, start: usize) -> Option<String> {
    let value = text.get(start..)?.strip_prefix("local:")?;
    let value: String = value
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .collect();
    (!value.is_empty()).then_some(value)
}

pub(super) fn find_word(text: &str, word: &str) -> Option<usize> {
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

pub(super) fn has_false_surfaced_thread_evidence(text: &str) -> bool {
    has_unnegated_phrase(text, "thread id did not surface", 16)
        || has_false_label_value(text, "thread id")
        || has_false_label_value(text, "surfaced thread id")
        || has_false_label_value(text, "observed thread id")
        || has_false_label_value(text, "resolved to thread")
        || has_false_label_value(text, "active owner")
        || has_unnegated_phrase(text, "thread did not surface", 16)
        || has_unnegated_phrase(text, "no thread surfaced", 16)
        || has_unnegated_phrase(text, "not surfaced", 16)
        || has_unnegated_phrase(text, "not visible", 16)
        || has_unnegated_phrase(text, "active owner: none", 16)
        || has_unnegated_phrase(text, "active owner = none", 16)
        || has_unnegated_phrase(text, "active owner: no", 16)
        || has_unnegated_phrase(text, "active owner unknown", 16)
        || has_unnegated_phrase(text, "owner thread unknown", 16)
        || has_unnegated_phrase(text, "remains unresolved", 16)
        || has_unnegated_phrase(text, "still unresolved", 16)
        || has_unnegated_phrase(text, "is unresolved", 16)
}

pub(super) fn has_false_bounded_search_evidence(text: &str) -> bool {
    has_unnegated_phrase(text, "not by branch", 16)
        || has_unnegated_phrase(text, "not by pr", 16)
        || has_unnegated_phrase(text, "not by pull request", 16)
        || has_unnegated_phrase(text, "not by issue", 16)
        || has_unnegated_phrase(text, "not by sha", 16)
        || has_unnegated_phrase(text, "not by commit", 16)
        || has_unnegated_phrase(text, "no branch search", 16)
        || has_unnegated_phrase(text, "no pr search", 16)
        || has_unnegated_phrase(text, "no sha search", 16)
        || has_unnegated_phrase(text, "without branch search", 16)
        || has_unnegated_phrase(text, "without pr search", 16)
        || has_unnegated_phrase(text, "without sha search", 16)
}

pub(super) fn has_nearby_negation(prefix: &str) -> bool {
    "no|no known|no longer|non|non-|not|not a|not an|isn't|is not|hasn't|without"
        .split('|')
        .any(|phrase| prefix.trim_end().ends_with(phrase))
}

pub(super) fn has_negated_pending_return(text: &str, start: usize, end: usize) -> bool {
    let prefix = &text[char_window_start(text, start, 32)..start].trim_end();
    let suffix = text[end..].trim_start();
    [
        "did not return a",
        "did not return any",
        "did not return",
        "returned no",
        "created no",
        "without returning a",
        "without returning any",
    ]
    .iter()
    .any(|phrase| prefix.ends_with(phrase))
        || [
            "was not returned",
            "was never returned",
            "never returned",
            "not returned",
        ]
        .iter()
        .any(|phrase| suffix.starts_with(phrase))
}

pub(super) fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}

fn has_true_value(value: &str) -> bool {
    "yes|true|allowed|safe|ok|okay"
        .split('|')
        .any(|word| value.strip_prefix(word).is_some_and(starts_with_boundary))
}

fn starts_with_boundary(rest: &str) -> bool {
    is_boundary(rest.chars().next())
}

fn has_explicit_false_value(remainder: &str) -> bool {
    let remainder = remainder.trim_start_matches([' ', '\t']);
    let Some(value) = remainder.strip_prefix([':', '=', '-', '?']) else {
        return false;
    };
    has_false_value(value.trim_start())
}

fn has_unsafe_decision_remainder(remainder: &str) -> bool {
    has_unnegated_phrase(remainder, "unsafe to reassign", 16)
        || has_unnegated_phrase(remainder, "unsafe to retry", 16)
        || has_unnegated_phrase(remainder, "unsafe reassignment", 16)
        || has_unnegated_phrase(remainder, "unsafe retry", 16)
        || has_unnegated_phrase(remainder, "not safe to reassign", 16)
        || has_unnegated_phrase(remainder, "not safe to retry", 16)
        || has_unnegated_phrase(remainder, "duplicate owners", 16)
}

fn is_terminal_decision_remainder(remainder: &str) -> bool {
    let mut chars = remainder.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    if first == '\n' || first == '\r' || matches!(first, ',' | '.' | ';' | '}') {
        return true;
    }
    if first != ' ' && first != '\t' {
        return false;
    }
    let remainder = remainder.trim_start_matches([' ', '\t']);
    remainder.is_empty() || remainder.starts_with(['\n', '\r', ',', '.', ';', '}'])
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

fn is_boundary(c: Option<char>) -> bool {
    c.is_none_or(|c| !c.is_ascii_alphanumeric())
}
