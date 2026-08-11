#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OrderedEvent {
    BlockedCall,
    ParentDirection,
    TerminalGoalCall,
    PackagedTerminalResult,
    Other,
}

pub(super) struct ActiveEvent {
    pub(super) line: String,
    pub(super) kind: OrderedEvent,
}

pub(super) fn active_events(evidence: &str) -> Vec<ActiveEvent> {
    let evidence = evidence.to_ascii_lowercase();
    super::super::child_lifecycle_events::active_lines(&evidence)
        .into_iter()
        .map(|line| {
            let kind = line
                .packaged_terminal
                .then_some(OrderedEvent::PackagedTerminalResult)
                .unwrap_or_else(|| ordered_event(&line.text));
            ActiveEvent {
                line: line.text,
                kind,
            }
        })
        .collect()
}

pub(super) fn ordered_event(line: &str) -> OrderedEvent {
    if line
        .strip_prefix("goal tool call: ")
        .and_then(|value| value.split(';').next())
        .is_some_and(super::super::child_terminal_handoff::is_blocked_goal_call)
    {
        OrderedEvent::BlockedCall
    } else if line.starts_with("parent direction event:") {
        OrderedEvent::ParentDirection
    } else if is_terminal_goal_call(line) {
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
    let words = value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let characters = words.iter().map(|word| word.chars().count()).sum::<usize>();
    let content = words
        .iter()
        .filter(|word| word.chars().count() >= 4 && word.chars().any(char::is_alphabetic))
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>();
    let concepts = content
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    (words.len() >= minimum_words
        && characters >= minimum_characters
        && concepts.len() >= minimum_concepts
        && (concepts.len() == content.len()
            || (content.len() >= 5
                && concepts.len() >= minimum_concepts + 2
                && concepts.len() * 5 >= content.len() * 4)))
        .then(|| concepts.into_iter().collect::<Vec<_>>().join("|"))
}
