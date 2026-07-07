pub(super) fn has_false_no_route_answer(clause: &str) -> bool {
    const NO_ROUTE_MARKERS: &str = "no fallback route was available|no fallback route available|no fallback path was available|no fallback path available|no alternate route was available|no alternate route available";
    NO_ROUTE_MARKERS
        .split('|')
        .filter_map(|marker| {
            clause
                .find(marker)
                .map(|index| &clause[index + marker.len()..])
        })
        .map(|answer| {
            answer.trim_start().trim_start_matches(|character: char| {
                character.is_ascii_whitespace()
                    || matches!(character, '?' | ':' | '=' | '-' | '\u{2013}' | '\u{2014}')
            })
        })
        .any(|answer| {
            ["no", "false"].into_iter().any(|negated_answer| {
                answer == negated_answer
                    || answer.strip_prefix(negated_answer).is_some_and(|rest| {
                        rest.starts_with(|character: char| !character.is_ascii_alphanumeric())
                    })
            })
        })
}
