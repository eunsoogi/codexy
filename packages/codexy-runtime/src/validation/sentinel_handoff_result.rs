use super::super::sentinel_handoff_status::{SentinelState, TerminalStatus};

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

fn events(text: &str) -> Vec<Event> {
    let mut events = Vec::new();
    let mut reviewer = None;
    for (start, segment) in super::result_context::segments(text) {
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

pub(super) fn active(text: &str, start: usize) -> bool {
    super::result_context::active(text, start)
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
