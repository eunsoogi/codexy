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
        if let Some(marker) = fence_info(trimmed) {
            match fence {
                None => fence = Some(marker),
                Some(open) if marker.0 == open.0 && marker.1 >= open.1 => fence = None,
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

fn fence_info(line: &str) -> Option<(u8, usize)> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = line.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some((marker, length))
}

pub(super) fn heading(line: &str) -> Option<(usize, &str)> {
    let line = line.trim_start();
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    (level > 0 && line.as_bytes().get(level) == Some(&b' ')).then(|| (level, line[level..].trim()))
}
