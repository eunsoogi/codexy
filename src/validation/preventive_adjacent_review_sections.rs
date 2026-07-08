pub(super) fn has_false_readiness_before_evidence(text: &str, start: usize) -> bool {
    ["readiness:", "pr readiness:", "pr ready:"]
        .iter()
        .any(|label| {
            text[..start].rfind(label).is_some_and(|index| {
                let value = text[index + label.len()..start]
                    .trim_start()
                    .trim_start_matches(['-', '*'])
                    .trim_start();
                ["not ", "false", "isn't ready", "aren't ready"]
                    .iter()
                    .any(|state| value.starts_with(state))
            })
        })
}

pub(super) fn has_readiness_not_applicable_state(text: &str) -> bool {
    ["pr readiness:", "readiness:"].iter().any(|label| {
        text.find(label).is_some_and(|index| {
            ["not applicable", "isn't applicable", "aren't applicable"]
                .iter()
                .any(|state| text[index + label.len()..].trim_start().starts_with(state))
        })
    })
}

pub(super) fn preventive_adjacent_review_end(text: &str, start: usize) -> usize {
    let suffix = &text[start..];
    let section_blank = suffix
        .match_indices("\n\n")
        .map(|(index, _)| index)
        .find(|index| !is_preventive_adjacent_heading_blank(suffix, *index));
    [section_blank, suffix.find("\n#"), suffix.find("\nreview ")]
        .into_iter()
        .flatten()
        .min()
        .map_or(text.len(), |index| start + index)
}

fn is_preventive_adjacent_heading_blank(suffix: &str, index: usize) -> bool {
    let heading = suffix[..index]
        .trim()
        .trim_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '#' | ':' | '-' | '.'));
    heading == "preventive adjacent review"
        || heading.starts_with("preventive adjacent review evidence")
}
