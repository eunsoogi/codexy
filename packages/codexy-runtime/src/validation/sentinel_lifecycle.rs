#[path = "sentinel_handoff_result_context.rs"]
mod result_context;

pub(super) const SENTINEL_MARKERS: &str = "sentinel|codexy-sentinel";

pub(super) fn packaged_terminal_result(text: &str) -> bool {
    result_context::packaged_terminal_result(text)
}

pub(super) fn active_result_line(text: &str, start: usize) -> bool {
    result_context::active(text, start)
}

pub(super) fn has_any(text: &str, phrases: &str) -> bool {
    phrases
        .split('|')
        .any(|item| affirmed_phrase_starts(text, item).next().is_some())
}
pub(super) fn affirmed_phrase_starts<'a>(
    text: &'a str,
    phrase: &'a str,
) -> impl Iterator<Item = usize> + 'a {
    let mut rest = text;
    let mut offset = 0;
    std::iter::from_fn(move || {
        while let Some(index) = rest.find(phrase) {
            let start = offset + index;
            let end = start + phrase.len();
            offset = end;
            rest = &text[offset..];
            if boundary(text[..start].chars().next_back()) && boundary(text[end..].chars().next()) {
                return Some(start);
            }
        }
        None
    })
}
pub(super) fn clause_bounds(text: &str, start: usize) -> (usize, usize) {
    let begin = text[..start]
        .rfind(['.', '!', '?', ';', ':', ',', '\n'])
        .map_or(0, |item| item + 1);
    let end = text[start..]
        .find(['.', '!', '?', ';', '\n'])
        .map_or(text.len(), |item| start + item);
    (begin, end)
}
fn boundary(value: Option<char>) -> bool {
    value.is_none_or(|item| !item.is_ascii_alphanumeric())
}
