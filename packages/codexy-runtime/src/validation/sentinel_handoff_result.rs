use super::super::sentinel_handoff_status::{SentinelState, TerminalStatus};
use std::collections::BTreeSet;

pub(super) enum Selection {
    Modeled(Option<(usize, SentinelState)>),
    ReviewerChanged,
}

struct Event {
    start: usize,
    status: SentinelState,
    prior: bool,
    reviewer: Option<String>,
}

pub(super) fn select(text: &str) -> Selection {
    let events = events(text);
    let mut reviewer = None;
    let mut selected = None;
    for event in events.iter().filter(|event| !event.prior) {
        if let Some(named) = event.reviewer.as_deref() {
            if reviewer.is_some_and(|active| active != named) {
                return Selection::ReviewerChanged;
            }
            reviewer = match event.status {
                SentinelState::Pending | SentinelState::Running => Some(named),
                SentinelState::Terminal(_) => None,
            };
            selected = Some((event.start, event.status));
        } else if vetoes(event.status) || selected.is_none() && is_pass(event.status) {
            selected = Some((event.start, event.status));
        }
    }
    Selection::Modeled(selected)
}

pub(super) fn active_terminal_result_lines(text: &str) -> BTreeSet<usize> {
    let mut end = 0;
    text.split_inclusive('\n')
        .enumerate()
        .filter_map(|(line, fragment)| {
            end += fragment.len();
            terminal_result_on_last_active_line(&text[..end]).then_some(line)
        })
        .collect()
}

fn terminal_result_on_last_active_line(text: &str) -> bool {
    let current = super::super::readiness_context::current_text(text);
    let current = current.trim_end_matches('\n');
    let last_line = current.rfind('\n').map_or(0, |index| index + 1);
    segments(&current).into_iter().any(|(start, segment)| {
        start >= last_line
            && statuses(segment).into_iter().any(|(offset, status)| {
                active(&current, start + offset) && matches!(status, SentinelState::Terminal(_))
            })
    })
}

fn events(text: &str) -> Vec<Event> {
    let mut events = Vec::new();
    let mut reviewer = None;
    for (start, segment) in segments(text) {
        for (offset, status) in statuses(segment) {
            if !active(text, start + offset) {
                continue;
            }
            let named = named_reviewer(segment)
                .or_else(|| lifecycle_reviewer(segment, status, reviewer.as_deref()));
            if let Some(named) = &named {
                reviewer = Some(named.clone());
            }
            events.push(Event {
                start: start + offset,
                status,
                prior: reviewer_or_run_history(segment),
                reviewer: named,
            });
        }
    }
    events
}

fn segments(text: &str) -> Vec<(usize, &str)> {
    let mut result = Vec::new();
    let mut start = 0;
    for (end, _) in text.match_indices(['.', '!', '?', ';', '\n']) {
        push(text, start, end, &mut result);
        start = end + 1;
    }
    push(text, start, text.len(), &mut result);
    result
}

fn push<'a>(text: &'a str, start: usize, end: usize, result: &mut Vec<(usize, &'a str)>) {
    let sentence = &text[start..end];
    let mut offset = 0;
    loop {
        let next = [" but ", " while ", " and ", ","]
            .iter()
            .filter_map(|delimiter| {
                sentence[offset..]
                    .find(delimiter)
                    .map(|index| (index, *delimiter))
            })
            .min_by_key(|(index, _)| *index);
        let Some((index, delimiter)) = next else {
            result.push((start + offset, &sentence[offset..]));
            return;
        };
        result.push((start + offset, &sentence[offset..offset + index]));
        offset += index + delimiter.len();
    }
}

pub(super) fn active(text: &str, start: usize) -> bool {
    let line_start = text[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[start..]
        .find('\n')
        .map_or(text.len(), |offset| start + offset);
    let line =
        super::super::readiness_context::without_container_prefix(&text[line_start..line_end]);
    !line.starts_with(['>', '"', '`', '~', '#'])
        && !line.starts_with("historical")
        && !line.starts_with("example")
        && (!under_stale_heading(text, start) || names_current_head(text, start))
        && text[..start]
            .lines()
            .filter(|line| {
                let line = super::super::readiness_context::without_container_prefix(line);
                line.starts_with("```") || line.starts_with("~~~")
            })
            .count()
            % 2
            == 0
}

fn under_stale_heading(text: &str, start: usize) -> bool {
    text[..start]
        .lines()
        .rev()
        .find_map(|line| {
            let heading = line
                .rsplit(['.', '!', '?', ';'])
                .next()
                .map(super::super::readiness_context::without_container_prefix)?;
            heading
                .strip_prefix('#')
                .map(|heading| heading.trim_start_matches('#').trim_start())
        })
        .is_some_and(super::super::readiness_context::is_stale)
}

fn statuses(segment: &str) -> Vec<(usize, SentinelState)> {
    let mut events = super::super::sentinel_handoff_status::marker_starts(segment);
    events.sort_by_key(|(start, _)| *start);
    events.dedup();
    events
}

fn vetoes(status: SentinelState) -> bool {
    matches!(
        status,
        SentinelState::Terminal(TerminalStatus::Block | TerminalStatus::Unobservable)
    )
}

fn is_pass(status: SentinelState) -> bool {
    matches!(status, SentinelState::Terminal(TerminalStatus::Pass))
}

fn names_current_head(text: &str, start: usize) -> bool {
    text[start..]
        .split(['.', '!', '?', ';', '\n'])
        .next()
        .is_some_and(|event| event.contains("current head") || event.contains("current pr head"))
}

fn named_reviewer(segment: &str) -> Option<String> {
    let identifiers: Vec<_> = identifier_tokens(segment).collect();
    let marker = identifiers.iter().position(|word| *word == "sentinel")?;
    identifiers
        .get(marker + 1)
        .filter(|word| identity(word))
        .map(|word| (*word).to_owned())
}

fn lifecycle_reviewer(segment: &str, status: SentinelState, prior: Option<&str>) -> Option<String> {
    let predicate = match status {
        SentinelState::Pending | SentinelState::Running => "is still running",
        SentinelState::Terminal(TerminalStatus::Unobservable) => "returned unobservable",
        SentinelState::Terminal(TerminalStatus::Pass) => "returned pass",
        SentinelState::Terminal(TerminalStatus::Block) => "returned block",
    };
    let prefix = segment
        .to_ascii_lowercase()
        .find(predicate)
        .map(|offset| &segment[..offset])?
        .trim_end();
    let lifecycle = matches!(status, SentinelState::Running)
        && (contains(segment, "wait") || contains(segment, "verdict") || contains(segment, "head"));
    if matches!(status, SentinelState::Running) && prefix.is_empty() && lifecycle {
        return prior.map(str::to_owned);
    }
    let reviewer = identifier_tokens(prefix).last()?;
    (identity(reviewer)
        && (!matches!(status, SentinelState::Pending | SentinelState::Running)
            || prior.is_some() && lifecycle))
        .then(|| reviewer.to_owned())
}

fn contains(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase().contains(needle)
}
fn reviewer_or_run_history(text: &str) -> bool {
    let tokens: Vec<_> = identifier_tokens(text).collect();
    tokens.iter().enumerate().any(|(index, token)| {
        ["prior", "previous", "earlier", "old", "initial"].contains(token)
            && tokens[index + 1..]
                .iter()
                .take(2)
                .any(|candidate| reviewer_or_run_qualifier(candidate))
    })
}
fn reviewer_or_run_qualifier(token: &str) -> bool {
    ["sentinel", "reviewer", "run"].contains(&token)
        || token.ends_with("-sentinel")
        || token.starts_with("reviewer-")
}
fn identifier_tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .filter(|word| !word.is_empty())
}
fn identity(word: &str) -> bool {
    word.len() >= 2
        && word.chars().any(|c| c.is_ascii_alphabetic())
        && ![
            "pass",
            "block",
            "unobservable",
            "reviewer",
            "gate",
            "run",
            "is",
            "still",
            "running",
        ]
        .contains(&word)
}
