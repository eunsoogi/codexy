use super::phrases::{blocked::ACTIVE_BLOCKERS_OR_REMAINING, negation};

pub(super) trait PhraseAlternatives {
    fn any_phrase(self, matcher: impl FnMut(&str) -> bool) -> bool;
}

impl PhraseAlternatives for &str {
    fn any_phrase(self, matcher: impl FnMut(&str) -> bool) -> bool {
        self.split('|').any(matcher)
    }
}

impl PhraseAlternatives for &[&str] {
    fn any_phrase(self, matcher: impl FnMut(&str) -> bool) -> bool {
        self.iter().copied().any(matcher)
    }
}

pub(super) fn has_any<P: PhraseAlternatives>(text: &str, phrases: P) -> bool {
    phrases.any_phrase(|phrase| has_unnegated_phrase(text, phrase, 16))
}

pub(super) fn has_unnegated_word(text: &str, word: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(word) {
        let absolute_index = offset + index;
        let after_index = absolute_index + word.len();
        if is_boundary(text[..absolute_index].chars().next_back())
            && is_boundary(text[after_index..].chars().next())
        {
            let prefix_start = char_window_start(text, absolute_index, negation_window);
            if !has_nearby_negation(&text[prefix_start..absolute_index])
                && !has_false_blocker_label(text, word, after_index)
            {
                return true;
            }
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}

fn has_false_blocker_label(text: &str, word: &str, after_index: usize) -> bool {
    if !matches!(word, "blocked" | "blocker" | "blockers") {
        return false;
    }
    let value = text[after_index..].trim_start();
    let value = value.strip_prefix("state").unwrap_or(value).trim_start();
    if !matches!(value.chars().next(), Some(':' | '-' | '?' | '=')) {
        return false;
    }
    let value = value[1..].trim_start();
    let first = value
        .split(|c: char| !matches!(c, '/' | '0'..='9' | 'a'..='z'))
        .next();
    let rest = first.map_or("", |f| value[f.len()..].trim_start_matches([' ', '\t']));
    let terminal = rest.chars().next().is_none_or(|c| ".;,\n\r".contains(c))
        || has_any(rest, ACTIVE_BLOCKERS_OR_REMAINING);
    matches!(first, Some("none" | "no" | "false" | "n/a" | "na")) && terminal
        || value.starts_with("not applicable")
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

fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

fn is_boundary(c: Option<char>) -> bool {
    c.is_none_or(|c| !c.is_ascii_alphanumeric())
}

fn has_nearby_negation(prefix: &str) -> bool {
    let prefix = prefix.trim_end();
    negation_phrase_matches(prefix)
        || prefix.rsplit_once(' ').is_some_and(|(before, word)| {
            negation::MODIFIERS.contains(&word) && negation_phrase_matches(before)
        })
}

fn negation_phrase_matches(prefix: &str) -> bool {
    negation::PHRASES
        .iter()
        .any(|phrase| prefix.ends_with(phrase))
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}
