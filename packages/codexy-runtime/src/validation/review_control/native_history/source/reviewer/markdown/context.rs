pub(super) fn line_ranges(raw: &str) -> Vec<(usize, usize, &str)> {
    let mut start = 0;
    let mut result = Vec::new();
    for part in raw.split_inclusive('\n') {
        let without_newline = part.strip_suffix('\n').unwrap_or(part);
        let line = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        result.push((start, start + line.len(), line));
        start += part.len();
    }
    if raw.is_empty() {
        result.push((0, 0, raw));
    }
    result
}

pub(super) fn operative_lines(raw: &str) -> Vec<(usize, usize, &str)> {
    let lines = line_ranges(raw);
    operative_line_indices(&lines)
        .into_iter()
        .map(|index| lines[index])
        .collect()
}

pub(super) fn operative_line_indices(lines: &[(usize, usize, &str)]) -> Vec<usize> {
    let mut fence = None;
    let mut result = Vec::new();
    for (index, (_, _, line)) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if let Some((marker, length, closes)) = fence_info(trimmed) {
            match fence {
                None => fence = Some((marker, length)),
                Some(open) if closes && marker == open.0 && length >= open.1 => fence = None,
                Some(_) => {}
            }
            continue;
        }
        if fence.is_none() && !trimmed.starts_with('>') {
            result.push(index);
        }
    }
    result
}

fn fence_info(line: &str) -> Option<(u8, usize, bool)> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = line.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some((marker, length, line[length..].trim().is_empty()))
}

pub(super) fn heading(line: &str) -> Option<(usize, &str)> {
    let line = line.trim_start();
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    (level > 0 && line.as_bytes().get(level) == Some(&b' ')).then(|| (level, line[level..].trim()))
}

pub(super) fn section_membership(
    lines: &[(usize, usize, &str)],
    is_operative: impl Fn(usize) -> bool,
    is_section: impl Fn(usize, &str) -> bool,
) -> Vec<bool> {
    let mut section_level = None;
    let mut result = vec![false; lines.len()];
    for (index, (_, _, line)) in lines.iter().enumerate() {
        if is_operative(index) {
            if let Some((level, title)) = heading(line) {
                if section_level.is_some_and(|active| level <= active) {
                    section_level = None;
                }
                if is_section(level, title) {
                    section_level = Some(level);
                }
            }
        }
        result[index] = section_level.is_some();
    }
    result
}
