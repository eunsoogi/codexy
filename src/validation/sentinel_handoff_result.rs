use super::super::sentinel_handoff_status::{SentinelState, TerminalStatus};

pub(super) enum Selection {
    Modeled(Option<(usize, SentinelState)>),
    ReviewerChanged,
    Unmodeled,
}

struct Unit {
    start: usize,
    status_offset: usize,
    status: SentinelState,
    prior: bool,
    reviewer: String,
}

pub(super) fn select(text: &str) -> Selection {
    let units = units(text);
    let current = units.iter().filter(|unit| !unit.prior).collect::<Vec<_>>();
    if current.len() < 2 && !units.iter().any(|unit| unit.prior) {
        return Selection::Unmodeled;
    }
    let Some(first) = current.first() else {
        return Selection::Modeled(None);
    };
    if current.iter().any(|unit| unit.reviewer != first.reviewer) {
        return Selection::ReviewerChanged;
    }
    Selection::Modeled(
        current
            .last()
            .map(|unit| (unit.start + unit.status_offset, unit.status)),
    )
}

fn units(text: &str) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut reviewer = None;
    for (start, segment) in segments(text) {
        if !active(text, start, segment) {
            continue;
        }
        let Some((status, status_offset)) = status(segment) else {
            continue;
        };
        let named = named_reviewer(segment)
            .or_else(|| lifecycle_reviewer(segment, status, reviewer.as_deref()));
        let Some(named) = named else {
            continue;
        };
        reviewer = Some(named.clone());
        units.push(Unit {
            start,
            status_offset,
            status,
            prior: contains_word(segment, "prior")
                || contains_word(segment, "previous")
                || contains_word(segment, "earlier")
                || contains_word(segment, "old")
                || contains_word(segment, "initial"),
            reviewer: named,
        });
    }
    units
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

fn active(text: &str, start: usize, segment: &str) -> bool {
    let trimmed = segment.trim_start();
    !trimmed.starts_with(['>', '"', '`', '~'])
        && !trimmed.starts_with("historical")
        && !trimmed.starts_with("example")
        && text[..start]
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                line.starts_with("```") || line.starts_with("~~~")
            })
            .count()
            % 2
            == 0
}

fn status(segment: &str) -> Option<(SentinelState, usize)> {
    if contains(segment, "unobservable")
        || contains(segment, "terminal tool failure")
        || contains(segment, "terminal reviewer failure")
    {
        Some((
            SentinelState::Terminal(TerminalStatus::Unobservable),
            status_offset(segment, "unobservable"),
        ))
    } else if contains(segment, "block") {
        Some((
            SentinelState::Terminal(TerminalStatus::Block),
            status_offset(segment, "block"),
        ))
    } else if contains(segment, "pass") {
        Some((
            SentinelState::Terminal(TerminalStatus::Pass),
            status_offset(segment, "pass"),
        ))
    } else if contains(segment, "still running") {
        Some((
            SentinelState::Running,
            status_offset(segment, "still running"),
        ))
    } else if contains(segment, "timed out") || contains(segment, "no verdict") {
        Some((SentinelState::Pending, status_offset(segment, "timed out")))
    } else {
        None
    }
}

fn status_offset(segment: &str, marker: &str) -> usize {
    segment.to_ascii_lowercase().find(marker).unwrap_or(0)
}

fn named_reviewer(segment: &str) -> Option<String> {
    let words: Vec<_> = words(segment).collect();
    let marker = words.iter().position(|word| *word == "sentinel")?;
    words
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
    let reviewer = words(prefix).last()?;
    (identity(reviewer)
        && (!matches!(status, SentinelState::Pending | SentinelState::Running)
            || prior.is_some() && lifecycle))
        .then(|| reviewer.to_owned())
}

fn contains(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase().contains(needle)
}
fn contains_word(text: &str, word: &str) -> bool {
    words(text).any(|candidate| candidate == word)
}
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
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
