use super::child_lane_classification_boundaries::lane_boundary;
use super::child_lane_classification_control::normalize_metadata_prefix;
use super::child_lane_ownership_phrases::metadata_key;
use super::workflow_profile_grammar::value_has_strict_signal;

pub(super) fn current_active_lines(evidence: &str) -> Vec<String> {
    let (mut fence, mut comment, mut code, mut lines) = (None, false, None, Vec::new());
    for raw in evidence.lines() {
        if fence.is_some() {
            if indented_code(raw) {
                lines.push(String::new());
                continue;
            }
            if let Some((marker, length, tail)) = fence_marker(raw) {
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
            lines.push(String::new());
            continue;
        }
        if !comment {
            let line = normalize_metadata_prefix(raw);
            if let Some((marker, length, _tail)) = fence_marker(line) {
                fence = Some((marker, length));
                lines.push(String::new());
                continue;
            }
        }
        let active = active_markdown(raw, &mut comment, &mut code);
        let line = normalize_metadata_prefix(&active);
        lines.push(line.to_owned());
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
    let mut columns = 0;
    for byte in line.bytes() {
        columns = match byte {
            b' ' => columns + 1,
            b'\t' => columns + (4 - columns % 4),
            _ => return columns >= 4,
        };
        if columns >= 4 {
            return true;
        }
    }
    false
}

fn active_markdown(line: &str, comment: &mut bool, code: &mut Option<usize>) -> String {
    let masked_code = without_inline_code(line, code);
    let mut line = masked_code.as_str();
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

fn without_inline_code(line: &str, code: &mut Option<usize>) -> String {
    if code.is_none() && invalid_fence_header(line) {
        return line.to_owned();
    }
    let mut visible = String::new();
    let mut rest = line;
    loop {
        if let Some(width) = *code {
            let Some(close) = matching_backticks(rest, width) else {
                visible.extend(std::iter::repeat_n(' ', rest.chars().count()));
                return visible;
            };
            let end = close + width;
            visible.extend(std::iter::repeat_n(' ', rest[..end].chars().count()));
            rest = &rest[end..];
            *code = None;
            continue;
        }
        let Some(open) = rest.find('`') else {
            visible.push_str(rest);
            return visible;
        };
        let width = rest[open..]
            .bytes()
            .take_while(|byte| *byte == b'`')
            .count();
        let content = &rest[open + width..];
        let Some(close) = matching_backticks(content, width) else {
            visible.push_str(&rest[..open]);
            visible.extend(std::iter::repeat_n(' ', rest[open..].chars().count()));
            *code = Some(width);
            return visible;
        };
        visible.push_str(&rest[..open]);
        visible.extend(std::iter::repeat_n(
            ' ',
            rest[open..open + width + close + width].chars().count(),
        ));
        rest = &content[close + width..];
    }
}

fn invalid_fence_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(marker) = trimmed.as_bytes().first() else {
        return false;
    };
    *marker == b'`'
        && trimmed.bytes().take_while(|byte| *byte == b'`').count() >= 3
        && fence_marker(line).is_none()
}

fn matching_backticks(text: &str, width: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index] == b'`' {
            index += 1;
        }
        if index - start == width {
            return Some(start);
        }
    }
    None
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
    let tail = &trimmed[length..];
    (length >= 3 && (marker != b'`' || !tail.contains('`'))).then_some((marker, length, tail))
}
