use super::super::sentinel_handoff_status::{SentinelState, StatusProvenance};

pub(super) fn packaged_terminal_result(text: &str) -> bool {
    packaged_status_events(text)
        .into_iter()
        .any(|(_, status)| matches!(status, SentinelState::Terminal(_)))
}

pub(super) fn active(text: &str, start: usize) -> bool {
    let line_start = text[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[start..]
        .find('\n')
        .map_or(text.len(), |offset| start + offset);
    let line =
        super::super::readiness_context::without_container_prefix(&text[line_start..line_end]);
    !line.starts_with(['>', '"', '`', '~', '#'])
        && (!inline_stale_heading(&text[..start])
            || names_current_head(text, start) && !inline_historical_heading(&text[..start]))
        && !line.starts_with("historical")
        && !line.starts_with("example")
        && (!under_stale_heading(text, start)
            || names_current_head(text, start) && !under_historical_heading(text, start))
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

fn inline_stale_heading(prefix: &str) -> bool {
    prefix
        .rsplit('#')
        .next()
        .is_some_and(|heading| super::super::readiness_context::is_stale(heading.trim_start()))
}

fn inline_historical_heading(prefix: &str) -> bool {
    prefix
        .rsplit('#')
        .next()
        .is_some_and(|heading| historical(heading.trim_start()))
}

fn under_stale_heading(text: &str, start: usize) -> bool {
    text[..start]
        .lines()
        .rev()
        .find_map(|line| {
            let heading = super::super::readiness_context::without_container_prefix(line);
            heading
                .strip_prefix('#')
                .map(|heading| heading.trim_start_matches('#').trim_start())
        })
        .is_some_and(super::super::readiness_context::is_stale)
}

fn under_historical_heading(text: &str, start: usize) -> bool {
    text[..start]
        .lines()
        .rev()
        .find_map(|line| {
            let heading = super::super::readiness_context::without_container_prefix(line);
            heading
                .strip_prefix('#')
                .map(|heading| heading.trim_start_matches('#').trim_start())
        })
        .is_some_and(historical)
}

fn historical(text: &str) -> bool {
    ["historical", "previous", "prior", "fallback"]
        .iter()
        .any(|prefix| {
            text.strip_prefix(prefix).is_some_and(|rest| {
                rest.chars()
                    .next()
                    .is_none_or(|character| !character.is_ascii_alphanumeric())
            })
        })
}

pub(super) fn names_current_head(text: &str, start: usize) -> bool {
    text[start..]
        .split(['.', '!', '?', ';', '\n'])
        .next()
        .is_some_and(|event| event.contains("current head") || event.contains("current pr head"))
}

fn packaged_status_events(segment: &str) -> Vec<(usize, SentinelState)> {
    let mut events = super::super::sentinel_handoff_status::marker_events(segment)
        .into_iter()
        .filter(|event| event.provenance == StatusProvenance::PackagedSentinel)
        .map(|event| (event.start, event.state))
        .collect::<Vec<_>>();
    events.sort_by_key(|(start, _)| *start);
    events.dedup();
    events
}
