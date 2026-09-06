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
    let mut in_fence = false;
    let mut result = Vec::new();
    for (start, end, line) in line_ranges(raw) {
        let trimmed = line.trim_start();
        if fence_marker(trimmed) {
            in_fence = !in_fence;
        } else if !in_fence && !trimmed.starts_with('>') {
            result.push((start, end, line));
        }
    }
    result
}

pub(super) fn fence_marker(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

pub(super) fn heading(line: &str) -> Option<(usize, &str)> {
    let line = line.trim_start();
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    (level > 0 && line.as_bytes().get(level) == Some(&b' ')).then(|| (level, line[level..].trim()))
}
