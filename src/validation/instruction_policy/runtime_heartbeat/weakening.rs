use super::markdown::contains_word;

pub(super) fn has_weakening_suffix(after: &str, conditional_markers: &[&str]) -> bool {
    let mut after = after;
    loop {
        after = after
            .trim_start_matches(|character: char| !character.is_alphanumeric() && character != '<');
        let Some(after_boundary) = after.strip_prefix("<markdown-boundary>") else {
            break;
        };
        after = after_boundary;
    }
    if leading_heading(after).is_some_and(|(heading, body)| {
        conditional_markers
            .iter()
            .any(|marker| heading.contains(marker.trim()))
            && (contains_word(body, "may") || body.contains("is not required"))
    }) {
        return true;
    }
    let after = after.trim_start_matches(|character: char| !character.is_alphanumeric());
    let (after, follows_adversative) = ["but", "however"]
        .iter()
        .find_map(|connector| {
            let remainder = after.strip_prefix(connector)?;
            remainder
                .chars()
                .next()
                .is_some_and(|character| !character.is_alphanumeric())
                .then_some(remainder)
        })
        .map_or((after, false), |remainder| {
            (
                remainder.trim_start_matches(|character: char| !character.is_alphanumeric()),
                true,
            )
        });
    [
        "unless ",
        "except ",
        "only if ",
        "may ",
        "is not required",
        "when possible",
        "if available",
        "as needed",
    ]
    .iter()
    .any(|marker| after.starts_with(marker))
        || follows_adversative
            && after
                .split(['.', ';'])
                .next()
                .is_some_and(|clause| contains_word(clause, "may"))
}

fn leading_heading(text: &str) -> Option<(&str, &str)> {
    let text =
        text.trim_start_matches(|character: char| !character.is_alphanumeric() && character != '<');
    let heading = text.strip_prefix("<markdown-heading>")?;
    let (heading, body) = heading.split_once("</markdown-heading>")?;
    Some((
        heading.trim(),
        body.split("<markdown-heading>").next().unwrap_or_default(),
    ))
}
