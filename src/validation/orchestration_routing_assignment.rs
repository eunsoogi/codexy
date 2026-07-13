pub(super) fn assignment_intent(segment: &str) -> bool {
    [
        " use ",
        " run on ",
        " run with ",
        " run using ",
        " set ",
        " assign",
        " receive",
        " remain ",
        " spawn",
        " request",
        " select ",
        " choose ",
    ]
    .iter()
    .any(|action| segment.contains(action))
        || segment.contains("model:")
        || segment.contains("reasoning_effort:")
}

pub(super) fn assigns_ultra(segment: &str) -> bool {
    segment.match_indices("ultra").any(|(index, _)| {
        is_token_at(segment, index, "ultra") && assignment_intent(&unquoted_prefix(segment, index))
    })
}

fn unquoted_prefix(text: &str, end: usize) -> String {
    let mut quote = None;
    text[..end]
        .chars()
        .filter(|character| match quote {
            Some(marker) if *character == marker => {
                quote = None;
                false
            }
            Some(_) => false,
            None if matches!(*character, '"' | '`') => {
                quote = Some(*character);
                false
            }
            None => true,
        })
        .collect()
}

fn is_token_at(text: &str, index: usize, token: &str) -> bool {
    let before = text[..index].chars().next_back();
    let after = text[index + token.len()..].chars().next();
    before.is_none_or(|character| !character.is_ascii_alphanumeric())
        && after.is_none_or(|character| !character.is_ascii_alphanumeric())
}
