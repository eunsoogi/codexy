pub(super) fn has_unnegated_different_lane_phrase(line: &str) -> bool {
    ["another", "different", "later", "other"]
        .into_iter()
        .any(|first| has_unnegated_phrase(line, first, "lane"))
}

fn has_unnegated_phrase(line: &str, first: &str, second: &str) -> bool {
    let tokens = line
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>();

    tokens.windows(2).enumerate().any(|(index, pair)| {
        pair[0] == first && pair[1] == second && !is_negated_phrase(&tokens, index)
    })
}

fn is_negated_phrase(tokens: &[String], phrase_start: usize) -> bool {
    if phrase_start == 0 {
        return false;
    }
    let window_start = phrase_start.saturating_sub(4);
    tokens[window_start..phrase_start]
        .iter()
        .any(|token| matches!(token.as_str(), "not" | "never" | "without"))
        || tokens[phrase_start - 1] == "no"
}
