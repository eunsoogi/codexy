#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OrderedEvent {
    BlockedCall,
    ParentDirection,
    TerminalGoalCall,
    TypedReviewTerminal,
    Other,
}

pub(super) struct ActiveEvent {
    pub(super) line: String,
    pub(super) kind: OrderedEvent,
}

use super::negation::{is_negation, is_token_character};

pub(super) fn active_events(plugin_root: &std::path::Path, evidence: &str) -> Vec<ActiveEvent> {
    super::super::child_lifecycle_events::active_lines(evidence)
        .into_iter()
        .map(|line| ActiveEvent {
            kind: ordered_event(plugin_root, &line.text),
            line: line.text,
        })
        .collect()
}

pub(super) fn ordered_event(plugin_root: &std::path::Path, line: &str) -> OrderedEvent {
    let line = line.to_ascii_lowercase();
    if line
        .strip_prefix("goal tool call: ")
        .and_then(|value| value.split(';').next())
        .is_some_and(super::super::child_terminal_handoff::is_blocked_goal_call)
    {
        OrderedEvent::BlockedCall
    } else if line.starts_with("parent direction event:") {
        OrderedEvent::ParentDirection
    } else if super::super::review_control::is_lifecycle_terminal(plugin_root, &line) {
        OrderedEvent::TypedReviewTerminal
    } else if is_terminal_goal_call(&line) {
        OrderedEvent::TerminalGoalCall
    } else {
        OrderedEvent::Other
    }
}

pub(super) fn is_blocked_pre_delivery(line: &str) -> bool {
    line.starts_with("parent goal pre-delivery:")
        && field(line, "operation")
            .is_some_and(super::super::child_terminal_handoff::is_blocked_goal_call)
}

pub(super) fn is_terminal_goal_call(line: &str) -> bool {
    line.strip_prefix("goal tool call: ")
        .and_then(|value| value.split(';').next())
        .is_some_and(super::super::child_terminal_handoff::is_terminal_goal_call)
}

pub(super) fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    let mut values = line
        .split_once(": ")
        .map_or(line, |(_, value)| value)
        .split(';')
        .map(str::trim)
        .filter_map(|part| part.strip_prefix(&prefix));
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

pub(super) fn has_distinct_substantive_values(
    line: &str,
    name: &str,
    minimum_values: usize,
    minimum_words: usize,
    minimum_characters: usize,
    minimum_concepts: usize,
) -> bool {
    field(line, name)
        .map(|value| {
            let identities = value
                .split('|')
                .map(str::trim)
                .map(|value| {
                    substantive_identity(value, minimum_words, minimum_characters, minimum_concepts)
                })
                .collect::<Option<Vec<_>>>();
            identities.is_some_and(|identities| {
                identities.len() >= minimum_values
                    && identities
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        == identities.len()
            })
        })
        .unwrap_or(false)
}

pub(super) fn is_substantive(
    value: &str,
    minimum_words: usize,
    minimum_characters: usize,
    minimum_concepts: usize,
) -> bool {
    substantive_identity(value, minimum_words, minimum_characters, minimum_concepts).is_some()
}

fn substantive_identity(
    value: &str,
    minimum_words: usize,
    minimum_characters: usize,
    minimum_concepts: usize,
) -> Option<String> {
    let tokens = value
        .split(|character: char| !is_token_character(character))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let words = tokens
        .iter()
        .map(|word| alphabetic_content(word))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let characters = words.iter().map(|word| word.chars().count()).sum::<usize>();
    let content = words
        .iter()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>();
    let long_content = content
        .iter()
        .filter(|word| word.chars().count() >= 4)
        .collect::<Vec<_>>();
    let concepts = content
        .iter()
        .filter(|word| word.chars().count() >= 4)
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let numeric_metadata = tokens
        .iter()
        .enumerate()
        .flat_map(|(index, word)| {
            let embedded = numeric_metadata_is_embedded(&tokens, index);
            numeric_runs(word).into_iter().filter(move |_| embedded)
        })
        .map(|number| format!("number:{number}"))
        .collect::<std::collections::BTreeSet<_>>();
    let negative = tokens.iter().any(|word| is_negation(word));
    let short_tokens = content.iter().filter(|word| word.chars().count() < 4);
    let repeated_short_tokens = short_tokens.clone().count().saturating_sub(
        short_tokens
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
    );
    let short_token_budget = concepts.len() / 2;
    (words.len() >= minimum_words
        && characters >= minimum_characters
        && repeated_short_tokens <= short_token_budget
        && concepts.len() >= minimum_concepts
        && (concepts.len() == long_content.len()
            || (long_content.len() >= 5
                && concepts.len() >= minimum_concepts + 2
                && concepts.len() * 5 >= long_content.len() * 4)))
        .then(|| {
            concepts
                .into_iter()
                .chain(numeric_metadata)
                .chain(negative.then_some("polarity:negative".to_owned()))
                .collect::<Vec<_>>()
                .join("|")
        })
}

fn numeric_metadata_is_embedded(tokens: &[&str], index: usize) -> bool {
    let lexical = alphabetic_content(tokens[index]);
    (!lexical.is_empty() && lexical.chars().count() >= 4)
        || (lexical.is_empty()
            && index > 0
            && index + 1 < tokens.len()
            && [tokens[index - 1], tokens[index + 1]]
                .into_iter()
                .all(|word| !alphabetic_content(word).is_empty()))
}

fn alphabetic_content(word: &str) -> String {
    word.chars()
        .filter(|character| character.is_alphabetic())
        .collect()
}

fn numeric_runs(word: &str) -> Vec<String> {
    word.split(|character: char| !character.is_numeric())
        .filter(|run| !run.is_empty())
        .map(str::to_owned)
        .collect()
}
