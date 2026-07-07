const ROUTE_PREFIX_TRIM_CHARACTERS: [char; 12] = [
    ',', ';', '.', '_', '-', '/', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}',
    '\u{2015}',
];

pub(super) fn has_substantive_route_value(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && !trimmed.starts_with("not ") && has_affirmative_route_event(trimmed)
}

fn has_affirmative_route_event(value: &str) -> bool {
    "parent sent|parent posted|parent delivered|parent routed|orchestrator sent|orchestrator posted|orchestrator delivered|orchestrator routed"
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
    let direct_segment = direct_route_segment(after_object);
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

fn direct_route_segment(after_object: &str) -> &str {
    const DIRECT_ROUTE_DELIMITERS: &str = " and then |, then |; then | then | and later checked |, later checked |; later checked | later checked | and later sent |, later sent |; later sent | later sent | and later posted |, later posted |; later posted | later posted | and later delivered |, later delivered |; later delivered | later delivered | and later routed |, later routed |; later routed | later routed | before checking |, before checking |; before checking | after checking |, after checking |; after checking | and subsequently checked |, subsequently checked |; subsequently checked | subsequently checked ";
    DIRECT_ROUTE_DELIMITERS
        .split('|')
        .filter_map(|delimiter| {
            after_object.find(delimiter).and_then(|index| {
                let before = &after_object[..index];
                let after = &after_object[index + delimiter.len()..];
                (!has_invalid_route_followup(after)).then_some((index, before))
            })
        })
        .min_by_key(|(index, _)| *index)
        .map(|(_, before)| before)
        .unwrap_or(after_object)
}

fn has_pre_action_route_negation(value: &str, action_index: usize) -> bool {
    let prefix = value[..action_index]
        .trim()
        .trim_end_matches(|character: char| {
            character.is_ascii_whitespace() || ROUTE_PREFIX_TRIM_CHARACTERS.contains(&character)
        })
        .trim();
    let local = prefix
        .rsplit([',', ';', '.'])
        .next()
        .unwrap_or(prefix)
        .trim()
        .trim_end_matches(':')
        .trim();
    let local = local.replace(&ROUTE_PREFIX_TRIM_CHARACTERS[3..], " ");
    let local = local.split_whitespace().collect::<Vec<_>>().join(" ");
    matches!(
        local.as_str(),
        "no" | "non"
            | "not"
            | "not an"
            | "not the"
            | "false"
            | "never"
            | "unable"
            | "it is false that"
            | "it is not true that"
            | "it is not the case that"
    ) || has_qualified_actor_negation(&local)
        || local.ends_with(" non")
        || local.ends_with(" false that")
        || local.starts_with("false positive")
        || local.starts_with("false-positive")
        || has_route_not_used_clause(&local)
}

fn has_qualified_actor_negation(local: &str) -> bool {
    let tokens = local.split_whitespace().collect::<Vec<_>>();
    let Some(negation_index) = tokens.iter().rposition(|token| *token == "not") else {
        return false;
    };
    let actor_prefix = strip_actor_article(&tokens[negation_index + 1..]);
    actor_prefix.is_empty()
        || actor_prefix.iter().all(|token| {
            matches!(
                *token,
                "actual"
                    | "assigned"
                    | "authorized"
                    | "correct"
                    | "expected"
                    | "intended"
                    | "real"
                    | "responsible"
                    | "same"
                    | "valid"
            )
        })
}

fn strip_actor_article<'a>(tokens: &'a [&'a str]) -> &'a [&'a str] {
    match tokens.first().copied() {
        Some("a" | "an" | "the") => &tokens[1..],
        _ => tokens,
    }
}

fn has_post_destination_route_negation(suffix: &str) -> bool {
    let suffix = suffix
        .trim_start()
        .trim_start_matches([',', ';', '.'])
        .trim_start();
    starts_with_negative_answer(suffix) || has_invalid_route_followup(suffix)
}

fn starts_with_negative_answer(suffix: &str) -> bool {
    let answer = suffix.trim_start().trim_start_matches(|character: char| {
        character.is_ascii_whitespace()
            || matches!(character, '?' | ':' | '=' | '-' | '\u{2013}' | '\u{2014}')
    });
    ["no", "false"].into_iter().any(|negated_answer| {
        answer == negated_answer
            || answer.strip_prefix(negated_answer).is_some_and(|rest| {
                rest.starts_with(|character: char| !character.is_ascii_alphanumeric())
            })
    })
}

fn has_invalid_route_followup(suffix: &str) -> bool {
    route_followup_clauses(suffix).any(has_invalid_route_followup_clause)
}

fn has_invalid_route_followup_clause(clause: &str) -> bool {
    has_failed_route_delivery_clause(clause) || has_route_not_used_clause(clause)
}

fn route_followup_clauses(suffix: &str) -> impl Iterator<Item = &str> {
    suffix
        .split([';', '.'])
        .flat_map(|clause| clause.split(" and then "))
        .flat_map(|clause| clause.split(" but "))
        .flat_map(|clause| clause.split(" although "))
        .flat_map(|clause| clause.split(" yet "))
        .flat_map(|clause| clause.split(" however "))
        .flat_map(|clause| clause.strip_prefix("then ").into_iter().chain([clause]))
        .flat_map(|clause| clause.strip_prefix("and ").into_iter().chain([clause]))
        .flat_map(|clause| clause.strip_prefix("but ").into_iter().chain([clause]))
        .flat_map(|clause| clause.strip_prefix("although ").into_iter().chain([clause]))
        .flat_map(|clause| clause.strip_prefix("yet ").into_iter().chain([clause]))
        .flat_map(|clause| clause.strip_prefix("however ").into_iter().chain([clause]))
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
}

fn has_failed_route_delivery_clause(clause: &str) -> bool {
    let normalized_clause;
    let clause = if clause.contains(&ROUTE_PREFIX_TRIM_CHARACTERS[3..]) {
        normalized_clause = clause.replace(&ROUTE_PREFIX_TRIM_CHARACTERS[3..], " ");
        normalized_clause.as_str()
    } else {
        clause
    };
    if matches!(clause, "failed" | "failure" | "failures") {
        return true;
    }
    ["failed", "failure", "failures"]
        .into_iter()
        .any(|failure| contains_phrase(clause, failure))
        && (has_failed_route_pronoun_clause(clause)
            || [
                "send",
                "sent",
                "sending",
                "post",
                "posted",
                "posting",
                "deliver",
                "delivered",
                "delivery",
                "route",
                "routed",
                "routing",
                "handoff",
                "message",
                "feedback",
            ]
            .into_iter()
            .any(|term| contains_phrase(clause, term)))
}

fn has_failed_route_pronoun_clause(clause: &str) -> bool {
    [
        "it failed",
        "that failed",
        "this failed",
        "the fallback failed",
    ]
    .into_iter()
    .any(|marker| contains_phrase(clause, marker))
}

fn has_route_not_used_clause(clause: &str) -> bool {
    [
        "not used",
        "was not used",
        "was not actually used",
        "was never used",
        "never used",
        "wasn't used",
        "isn't used",
        "did not use",
        "didn't use",
        "unused",
    ]
    .into_iter()
    .any(|marker| contains_phrase(clause, marker))
}

fn has_phrase_boundaries(value: &str, start: usize, phrase: &str) -> bool {
    let end = start + phrase.len();
    value[..start]
        .chars()
        .last()
        .is_none_or(|character| !is_route_word_character(character))
        && value[end..]
            .chars()
            .next()
            .is_none_or(|character| !is_route_word_character(character))
}

fn contains_phrase(value: &str, phrase: &str) -> bool {
    value
        .find(phrase)
        .is_some_and(|index| has_phrase_boundaries(value, index, phrase))
}

fn is_route_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || ROUTE_PREFIX_TRIM_CHARACTERS[3..].contains(&character)
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
