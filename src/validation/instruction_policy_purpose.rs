pub(super) fn has_prohibition_marker(lower: &str, marker: &str) -> bool {
    contains_wordish(lower, marker)
        && lower.match_indices(marker).any(|(index, _)| {
            !lower[..index].trim_end().ends_with("must not")
                && !allows_prohibition_marker(lower, index)
        })
}

pub(super) fn allows_prohibition_marker(lower: &str, marker_index: usize) -> bool {
    let Some(modal_index) = last_modal_before(lower, marker_index) else {
        return false;
    };
    let context = &lower[modal_index + "must".len()..marker_index];
    [
        " if ",
        " so ",
        " so that ",
        " to ",
        " in order to ",
        " when ",
    ]
    .iter()
    .any(|connector| context.contains(connector))
}

fn last_modal_before(lower: &str, marker_index: usize) -> Option<usize> {
    let mut modal_index = None;
    for (index, _) in lower[..marker_index].match_indices("must") {
        if is_word_boundary(lower, index, "must") {
            modal_index = Some(index);
        }
    }
    modal_index
}

fn is_word_boundary(text: &str, index: usize, word: &str) -> bool {
    let before = index
        .checked_sub(1)
        .and_then(|before| text.as_bytes().get(before));
    let after = text.as_bytes().get(index + word.len());
    before.is_none_or(|byte| !byte.is_ascii_alphanumeric())
        && after.is_none_or(|byte| !byte.is_ascii_alphanumeric())
}

fn contains_wordish(text: &str, marker: &str) -> bool {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '\''))
        .collect::<Vec<_>>()
        .windows(marker.split_whitespace().count())
        .any(|window| window.join(" ") == marker)
}
