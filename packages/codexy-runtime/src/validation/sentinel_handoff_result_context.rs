use std::collections::BTreeSet;

use super::super::sentinel_handoff_status::{SentinelState, StatusProvenance};

pub(super) fn active_packaged_terminal_result_lines(text: &str) -> BTreeSet<usize> {
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
            && packaged_status_events(segment)
                .into_iter()
                .any(|(offset, status)| {
                    active(&current, start + offset) && matches!(status, SentinelState::Terminal(_))
                })
    })
}

pub(super) fn segments(text: &str) -> Vec<(usize, &str)> {
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
        && (!inline_stale_heading(&text[..start]) || names_current_head(text, start))
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

fn inline_stale_heading(prefix: &str) -> bool {
    prefix
        .rsplit('#')
        .next()
        .is_some_and(|heading| super::super::readiness_context::is_stale(heading.trim_start()))
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
