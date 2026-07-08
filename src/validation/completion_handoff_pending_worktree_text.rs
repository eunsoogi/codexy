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
            if ["is allowed", "allowed", "is safe", "safe"]
                .iter()
                .any(|value| suffix.strip_prefix(value).is_some_and(starts_with_boundary))
            {
                return true;
            }
            if let Some(value) = suffix.strip_prefix([':', '=', '-', '?']) {
                let value = value.trim_start();
                if has_true_value(value) {
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

pub(super) fn has_nearby_negation(prefix: &str) -> bool {
    "no|no known|no longer|non|non-|not|not a|not an|isn't|is not|hasn't|without"
        .split('|')
        .any(|phrase| prefix.trim_end().ends_with(phrase))
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
