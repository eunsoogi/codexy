use std::collections::BTreeSet;

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
    let terminal_lines =
        super::super::sentinel_handoff::active_packaged_terminal_result_lines(&evidence);
    let mut events = Vec::new();
    let mut start = 0;
    for (source_line, fragment) in evidence.split_inclusive('\n').enumerate() {
        if super::super::sentinel_handoff::active_result_line(&evidence, start) {
            let line = without_list_prefix(fragment.trim()).to_owned();
            let kind = terminal_lines
                .contains(&source_line)
                .then_some(OrderedEvent::PackagedTerminalResult)
                .unwrap_or_else(|| ordered_event(&line));
            events.push(ActiveEvent { line, kind });
        }
        start += fragment.len();
    }
    events
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

pub(super) fn has_distinct_values(line: &str, name: &str, minimum: usize) -> bool {
    field(line, name)
        .map(|value| {
            value
                .split('|')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<BTreeSet<_>>()
                .len()
                >= minimum
        })
        .unwrap_or(false)
}

pub(super) fn has_elapsed_minimum(line: &str) -> bool {
    let parse = |name| field(line, name).and_then(|value| value.parse::<u64>().ok());
    parse("first monotonic ms")
        .zip(parse("observed monotonic ms"))
        .zip(parse("minimum interval ms"))
        .is_some_and(|((first, observed), minimum)| {
            minimum > 0 && observed.saturating_sub(first) >= minimum
        })
}

fn without_list_prefix(mut line: &str) -> &str {
    loop {
        line = line.trim_start();
        if let Some(rest) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("+ "))
        {
            line = rest;
            continue;
        }
        let numbered = line.trim_start_matches(|character: char| character.is_ascii_digit());
        if numbered.len() != line.len() {
            if let Some(rest) = numbered
                .strip_prefix(". ")
                .or_else(|| numbered.strip_prefix(") "))
            {
                line = rest;
                continue;
            }
        }
        return line;
    }
}
