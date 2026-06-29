pub(super) fn has_unnegated_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(start, _)| {
        let prefix_start = char_window_start(text, start, 16);
        let prefix = &text[prefix_start..start];
        is_boundary(text[..start].chars().next_back())
            && is_boundary(text[start + phrase.len()..].chars().next())
            && !has_nearby_negation(prefix)
    })
}

pub(super) fn has_unnegated_readiness_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(start, _)| {
        let end = start + phrase.len();
        is_boundary(text[..start].chars().next_back())
            && is_boundary(text[end..].chars().next())
            && !has_nearby_negation(&text[char_window_start(text, start, 16)..start])
            && !has_negative_label_value(&text[end..])
    })
}

pub(super) fn has_unnegated_action_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(start, _)| {
        let prefix_start = char_window_start(text, start, 16);
        let prefix = &text[prefix_start..start];
        let end = start + phrase.len();
        is_action_boundary(text[..start].chars().next_back())
            && is_action_suffix_boundary(text[end..].chars())
            && !has_nearby_negation(prefix)
    })
}

fn has_negative_label_value(suffix: &str) -> bool {
    let Some(value) = label_value(suffix) else {
        return false;
    };
    "not ready|not yet ready|not currently ready|isn't ready|isn't yet ready|isn't currently ready|aren't ready|aren't yet ready|aren't currently ready|false|not requested|isn't requested|aren't requested|not applicable|isn't applicable|aren't applicable"
        .split('|')
        .any(|phrase| value.strip_prefix(phrase).is_some_and(starts_with_boundary))
        || value
            .strip_prefix("no")
            .is_some_and(starts_with_standalone_label_boundary)
}

fn label_value(suffix: &str) -> Option<&str> {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let value = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('?'))?;
    Some(value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']))
}

fn has_nearby_negation(prefix: &str) -> bool {
    "no|not|not yet|not currently|without|neither|isn't|isn't yet|isn't currently|aren't|aren't yet|aren't currently|wasn't|wasn't yet|weren't|weren't yet|has not been|has not yet been|hasn't|hasn't yet|hasn't been|hasn't yet been|have not been|have not yet been|haven't|haven't yet|haven't been|haven't yet been|had not been|had not yet been|hadn't|hadn't yet|hadn't been|hadn't yet been|can't|can't yet|cannot|cannot yet|won't|won't yet|don't|don't yet|doesn't|doesn't yet|didn't|didn't yet"
        .split('|')
        .any(|term| prefix.trim_end().ends_with(term))
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn is_action_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        !character.is_ascii_alphanumeric() && !matches!(character, '_' | '-' | '/' | '.')
    })
}

fn is_action_suffix_boundary(mut characters: impl Iterator<Item = char>) -> bool {
    match characters.next() {
        Some('.') => is_boundary(characters.next()),
        character => is_action_boundary(character),
    }
}

fn starts_with_boundary(rest: &str) -> bool {
    is_boundary(rest.chars().next())
}

fn starts_with_standalone_label_boundary(rest: &str) -> bool {
    rest.is_empty()
        || rest
            .chars()
            .next()
            .is_some_and(|character| matches!(character, '.' | ';' | ',' | '\n' | '\r'))
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}
