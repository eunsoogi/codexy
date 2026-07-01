pub(super) fn has_substantive_route_value(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && !trimmed.starts_with("not ") && has_affirmative_route_event(trimmed)
}

fn has_affirmative_route_event(value: &str) -> bool {
    "parent sent|parent posted|parent delivered|parent routed"
        .split('|')
        .any(|action| {
            value.match_indices(action).any(|(index, _)| {
                has_phrase_boundaries(value, index, action)
                    && !has_pre_action_route_negation(value, index)
                    && has_routed_object(&value[index + action.len()..])
            })
        })
}

fn has_routed_object(after_action: &str) -> bool {
    "handoff|message|feedback".split('|').any(|object| {
        after_action.find(object).is_some_and(|index| {
            has_phrase_boundaries(after_action, index, object)
                && !has_route_event_negation(&after_action[..index])
                && has_positive_destination(&after_action[index + object.len()..])
        })
    })
}

fn has_positive_destination(after_object: &str) -> bool {
    let direct_segment = after_object
        .split(" and then ")
        .next()
        .unwrap_or(after_object);
    "to the child thread|in the child thread|into the child thread|via the child thread|through the child thread|at the child thread|to the child owner|at the child owner|to the reviewer"
        .split('|')
        .filter_map(|destination| direct_segment.find(destination).map(|index| (index, destination)))
        .any(|(index, destination)| {
            let prefix = &direct_segment[..index];
            let suffix = &direct_segment[index + destination.len()..];
            has_phrase_boundaries(direct_segment, index, destination)
                && !has_route_event_negation(prefix)
                && !has_post_destination_route_negation(suffix)
                && !"other than|rather than|instead of|except"
                    .split('|')
                    .any(|marker| prefix.contains(marker))
        })
}

fn has_pre_action_route_negation(value: &str, action_index: usize) -> bool {
    let prefix = value[..action_index]
        .trim()
        .trim_end_matches([',', ';', '.', '-'])
        .trim();
    let local = prefix
        .rsplit([',', ';', '.'])
        .next()
        .unwrap_or(prefix)
        .trim()
        .trim_end_matches(':')
        .trim();
    matches!(
        local,
        "no" | "false" | "never" | "unable" | "it is false that"
    ) || local.ends_with(" false that")
}

fn has_post_destination_route_negation(suffix: &str) -> bool {
    let suffix = suffix
        .trim_start()
        .trim_start_matches([',', ';', '.'])
        .trim_start();
    suffix.starts_with("? no")
        || [
            "? no",
            "not used",
            "was not used",
            "was not actually used",
            "was never used",
            "never used",
            "wasn't used",
        ]
        .into_iter()
        .any(|marker| suffix.contains(marker))
}

fn has_phrase_boundaries(value: &str, start: usize, phrase: &str) -> bool {
    let end = start + phrase.len();
    value[..start]
        .chars()
        .last()
        .is_none_or(|character| !character.is_ascii_alphanumeric())
        && value[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn has_route_event_negation(text: &str) -> bool {
    text.split_whitespace().map(route_word_token).any(|token| {
        matches!(
            token,
            "failed" | "no" | "not" | "never" | "unable" | "without" | "cannot"
        ) || token.ends_with("n't")
    })
}

fn route_word_token(word: &str) -> &str {
    word.trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
}
