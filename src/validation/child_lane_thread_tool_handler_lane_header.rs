pub(super) fn include_preceding_lane_header(evidence: &str, start: usize) -> usize {
    let mut cursor = start;
    while let Some(previous_end) = cursor.checked_sub(1) {
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous_line = &evidence[previous_start..previous_end];
        if previous_line.trim().is_empty() {
            return start;
        }
        if let Some(header_label) = lane_header_label(previous_line) {
            if block_has_conflicting_lane_label(evidence, start, header_label.as_str()) {
                return start;
            }
            return previous_start;
        }
        cursor = previous_start;
    }
    start
}

fn lane_header_label(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let line = strip_markdown_heading_prefix(trimmed).trim();
    let Some(rest) = line
        .strip_prefix("lane ")
        .or_else(|| line.strip_prefix("Lane "))
    else {
        return None;
    };
    let label = rest.trim_end_matches([':', '.']).trim();
    ((trimmed != line || line.ends_with(':'))
        && !label.is_empty()
        && label.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    .then(|| format!("lane {}", label.to_ascii_lowercase()))
}

fn block_has_conflicting_lane_label(evidence: &str, start: usize, header_label: &str) -> bool {
    let mut cursor = start;
    while cursor < evidence.len() {
        let line_end = evidence[cursor..]
            .find('\n')
            .map_or(evidence.len(), |index| cursor + index);
        let line = &evidence[cursor..line_end];
        if line.trim().is_empty() {
            return false;
        }
        if line_starts_with_lane_label(line).is_some_and(|label| label != header_label) {
            return true;
        }
        if line_end == evidence.len() {
            return false;
        }
        cursor = line_end + 1;
    }
    false
}

fn line_starts_with_lane_label(line: &str) -> Option<String> {
    let line = strip_markdown_heading_prefix(line.trim_start()).trim_start();
    let rest = line
        .strip_prefix("lane ")
        .or_else(|| line.strip_prefix("Lane "))?;
    let label = rest
        .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .next()
        .unwrap_or_default()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    (!label.is_empty() && label.bytes().all(|byte| byte.is_ascii_alphanumeric()))
        .then(|| format!("lane {}", label.to_ascii_lowercase()))
}

fn strip_markdown_heading_prefix(line: &str) -> &str {
    let marker_end = line.bytes().take_while(|byte| *byte == b'#').count();
    if marker_end > 0 && line[marker_end..].starts_with(' ') {
        line[marker_end..].trim_start()
    } else {
        line
    }
}
