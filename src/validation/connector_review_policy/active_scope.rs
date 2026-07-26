pub(super) fn lines(text: &str) -> Vec<String> {
    let mut active = Vec::new();
    let mut fence = None;
    let mut inactive_at = None;
    for raw in text.lines() {
        let trimmed = raw.trim();
        if let Some((marker, count, rest)) = fence_delimiter(trimmed) {
            fence = match fence {
                Some((opened_marker, minimum))
                    if marker == opened_marker && count >= minimum && rest.trim().is_empty() =>
                {
                    None
                }
                Some(opened) => Some(opened),
                None => Some((marker, count)),
            };
            continue;
        }
        if fence.is_some()
            || raw.starts_with("    ")
            || raw.starts_with('\t')
            || trimmed.starts_with('>')
        {
            continue;
        }
        if let Some((level, title)) = heading(trimmed) {
            if inactive_at.is_some_and(|ancestor| level <= ancestor) {
                inactive_at = None;
            }
            if inactive_title(title) {
                inactive_at = Some(level);
            }
            if inactive_at.is_none() {
                active.push(trimmed.to_owned());
            }
            continue;
        }
        if inactive_at.is_none()
            && !quoted_or_example(without_list_prefix(trimmed))
            && !trimmed.is_empty()
        {
            active.push(trimmed.to_owned());
        }
    }
    active
}

fn fence_delimiter(line: &str) -> Option<(char, usize, &str)> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let count = line
        .chars()
        .take_while(|candidate| *candidate == marker)
        .count();
    (count >= 3).then(|| (marker, count, &line[count..]))
}

fn without_list_prefix(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix(['-', '*', '+']) {
        return rest.trim_start();
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && matches!(line.as_bytes().get(digits), Some(b'.' | b')')) {
        return line[digits + 1..].trim_start();
    }
    line
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    (level > 0 && line.as_bytes().get(level) == Some(&b' '))
        .then_some((level, line[level..].trim()))
}

fn inactive_title(title: &str) -> bool {
    let title = title.to_ascii_lowercase();
    ["quoted", "example", "historical", "history", "inactive"]
        .iter()
        .any(|marker| title.starts_with(marker))
}

fn quoted_or_example(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.starts_with(['"', '\'', '“'])
        || lower.starts_with("example:")
        || lower.starts_with("quoted:")
        || lower.starts_with("historical:")
        || lower.starts_with("history:")
        || lower.starts_with("inactive:")
}
