pub(super) fn lines(text: &str) -> Vec<String> {
    let mut active = Vec::new();
    let mut fence = false;
    let mut inactive_at = None;
    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = !fence;
            continue;
        }
        if fence || raw.starts_with("    ") || raw.starts_with('\t') || trimmed.starts_with('>') {
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
        if inactive_at.is_none() && !quoted_or_example(trimmed) && !trimmed.is_empty() {
            active.push(trimmed.to_owned());
        }
    }
    active
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
