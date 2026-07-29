use super::child_lane_classification_boundaries::lane_boundary;
use super::child_lane_classification_control::normalize_metadata_prefix;
use super::child_lane_ownership_phrases::metadata_key;
use super::workflow_profile_grammar::value_has_strict_signal;

pub(super) fn current_active_lines(evidence: &str) -> Vec<String> {
    let (mut fence, mut comment, mut lines) = (None, false, Vec::new());
    for raw in evidence.lines() {
        if fence.is_some() {
            let line = normalize_metadata_prefix(raw);
            if let Some((marker, length, tail)) = fence_marker(line) {
                if fence.is_some_and(|(open, minimum)| {
                    marker == open && length >= minimum && tail.trim().is_empty()
                }) {
                    fence = None;
                }
            }
            lines.push(String::new());
            continue;
        }
        if indented_code(raw) {
            active_markdown(raw, &mut comment);
            lines.push(String::new());
            continue;
        }
        let active = active_markdown(raw, &mut comment);
        let line = normalize_metadata_prefix(&active);
        if let Some((marker, length, _tail)) = fence_marker(line) {
            fence = Some((marker, length));
            lines.push(String::new());
        } else {
            lines.push(line.to_owned());
        }
    }
    let references = lines.iter().map(String::as_str).collect::<Vec<_>>();
    let start = (0..references.len())
        .rev()
        .find(|index| {
            lane_boundary(&references, *index)
                .is_some_and(|boundary| boundary.resets_authority_record())
        })
        .map_or(0, |index| index + 1);
    lines.into_iter().skip(start).collect()
}

fn indented_code(line: &str) -> bool {
    line.starts_with('\t') || line.bytes().take_while(|byte| *byte == b' ').count() >= 4
}

fn active_markdown<'a>(mut line: &'a str, comment: &mut bool) -> String {
    let mut active = String::new();
    loop {
        if *comment {
            let Some(end) = line.find("-->") else {
                return active;
            };
            *comment = false;
            line = &line[end + 3..];
        } else {
            let Some(start) = line.find("<!--") else {
                active.push_str(line);
                return active;
            };
            active.push_str(&line[..start]);
            *comment = true;
            line = &line[start + 4..];
        }
    }
}

pub(super) fn has_strict_work_signal(lines: &[&str]) -> bool {
    lines
        .iter()
        .filter_map(|line| {
            ["task kind", "lane type", "risk"]
                .iter()
                .find_map(|field| field_value(line, field))
        })
        .any(value_has_strict_signal)
}

pub(super) fn field_value<'a>(line: &'a str, expected: &str) -> Option<&'a str> {
    line.split_once(':')
        .and_then(|(key, value)| (metadata_key(key) == expected).then_some(value.trim()))
}

fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let trimmed = line.trim_start();
    let marker = *trimmed.as_bytes().first()?;
    (marker == b'`' || marker == b'~').then_some(())?;
    let length = trimmed.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some((marker, length, &trimmed[length..]))
}
