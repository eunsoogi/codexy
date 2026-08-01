use super::child_lane_classification_boundaries::lane_boundary;
use super::child_lane_classification_control::normalize_metadata_prefix;
use super::child_lane_ownership_phrases::metadata_key;
use super::workflow_profile_grammar::value_has_strict_signal;

pub(super) fn current_active_lines(evidence: &str) -> Vec<String> {
    let raw_lines = evidence.lines().collect::<Vec<_>>();
    let (mut fence, mut state, mut lines) = (None, MarkdownState::default(), Vec::new());
    for (index, raw) in raw_lines.iter().enumerate() {
        if fence.is_some() {
            if !active_markdown_line(raw) {
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
        if state.code.is_some() && inline_code_block_boundary(raw) {
            state.code = None;
        }
        if !active_markdown_line(raw) {
            lines.push(String::new());
            continue;
        }
        let block_boundary = lines.is_empty()
            || lines
                .last()
                .is_some_and(|line| line.is_empty() || atx_heading(line));
        if !state.comment {
            let line = markdown_block_prefix(raw, block_boundary);
            if let Some((marker, length, _tail)) = fence_marker(line) {
                fence = Some((marker, length));
                lines.push(String::new());
                continue;
            }
        }
        let active = active_markdown(raw, &raw_lines[index + 1..], &mut state);
        let line = markdown_block_prefix(&active, block_boundary);
        let line = normalize_metadata_prefix(markdown_block_prefix(line, false));
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

#[derive(Default)]
struct MarkdownState {
    comment: bool,
    code: Option<usize>,
}

fn active_markdown_line(line: &str) -> bool {
    !indented_code(line)
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

fn active_markdown(line: &str, later: &[&str], state: &mut MarkdownState) -> String {
    if !state.comment && state.code.is_none() && invalid_fence_header(line) {
        return line.to_owned();
    }
    let mut line = line;
    let mut active = String::new();
    loop {
        if state.comment {
            let Some(end) = line.find("-->") else {
                return active;
            };
            state.comment = false;
            line = &line[end + 3..];
        } else if let Some(width) = state.code {
            let Some(close) = matching_backticks(line, width) else {
                return active;
            };
            state.code = None;
            line = &line[close + width..];
        } else {
            let comment = line.find("<!--");
            let code = line.find('`');
            let Some(start) = [comment, code].into_iter().flatten().min() else {
                active.push_str(line);
                return active;
            };
            active.push_str(&line[..start]);
            if comment == Some(start) {
                state.comment = true;
                line = &line[start + 4..];
            } else {
                let width = line[start..]
                    .bytes()
                    .take_while(|byte| *byte == b'`')
                    .count();
                let rest = &line[start + width..];
                if let Some(close) = matching_backticks(rest, width) {
                    line = &rest[close + width..];
                } else if closes_later(later, width) {
                    state.code = Some(width);
                    return active;
                } else {
                    active.push_str(&line[start..]);
                    return active;
                }
            }
        }
    }
}

fn closes_later(lines: &[&str], width: usize) -> bool {
    for line in lines {
        if !active_markdown_line(line) {
            continue;
        }
        if inline_code_block_boundary(line) {
            return false;
        }
        if matching_backticks(line, width).is_some() {
            return true;
        }
    }
    false
}

fn inline_code_block_boundary(line: &str) -> bool {
    if line.trim().is_empty() {
        return true;
    }
    if !active_markdown_line(line) {
        return false;
    }
    let normalized = markdown_block_prefix(line, false);
    normalized != line.trim_start() || atx_heading(normalized) || fence_marker(normalized).is_some()
}

fn markdown_block_prefix(line: &str, blank_precedes: bool) -> &str {
    let line = line.trim_start();
    let bytes = line.as_bytes();
    if matches!(bytes.first(), Some(b'-' | b'+' | b'*'))
        && matches!(bytes.get(1), Some(b' ' | b'\t'))
    {
        return &line[2..];
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if (1..=9).contains(&digits)
        && (blank_precedes || line[..digits].parse::<u32>() == Ok(1))
        && matches!(bytes.get(digits), Some(b'.' | b')'))
        && matches!(bytes.get(digits + 1), Some(b' ' | b'\t'))
    {
        return &line[digits + 2..];
    }
    line
}

fn atx_heading(line: &str) -> bool {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return false;
    }
    let line = &line[indent..];
    let markers = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6).contains(&markers) && matches!(line.as_bytes().get(markers), None | Some(b' ' | b'\t'))
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
