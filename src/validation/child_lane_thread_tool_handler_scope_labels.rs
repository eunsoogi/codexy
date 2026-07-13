pub(super) fn line_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = strip_list_prefix(line);
    let (key, value) = trimmed.split_once(':')?;
    let key = strip_lane_label_prefix(key);
    if key.trim().is_empty() {
        return value.trim_start().split_once(':');
    }
    Some((key, value))
}

pub(super) fn strip_list_prefix(line: &str) -> &str {
    let line = line.trim_start();
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return rest.trim_start();
    }
    let Some((marker, rest)) = line.split_once(['.', ')']) else {
        return line;
    };
    if !marker.is_empty() && marker.bytes().all(|byte| byte.is_ascii_digit()) {
        rest.trim_start()
    } else {
        line
    }
}

fn strip_lane_label_prefix(key: &str) -> &str {
    let Some(rest) = key
        .trim_start()
        .strip_prefix("Lane ")
        .or_else(|| key.trim_start().strip_prefix("lane "))
    else {
        return key;
    };
    let label_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '-' || ch == '.')
        .unwrap_or(rest.len());
    let label = rest[..label_end].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    if label.is_empty() {
        return key;
    }
    rest[label_end..].trim_start_matches(|ch: char| ch.is_whitespace() || ch == '-' || ch == '.')
}

pub(super) fn lane_label(line: &str) -> Option<String> {
    let trimmed = strip_markdown_heading_prefix(strip_list_prefix(line));
    let rest = trimmed
        .strip_prefix("Lane ")
        .or_else(|| trimmed.strip_prefix("lane "))?;
    let label = rest
        .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .next()
        .unwrap_or_default()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    (!label.is_empty() && !label.eq_ignore_ascii_case("ownership"))
        .then(|| format!("lane {}", label.to_ascii_lowercase()))
}

fn strip_markdown_heading_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    let marker_end = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if marker_end > 0 && trimmed[marker_end..].starts_with(' ') {
        trimmed[marker_end..].trim_start()
    } else {
        line
    }
}

pub(super) fn is_handler_capture_line(line: &str) -> bool {
    "captured|classified|recorded|reported|routed|tracked"
        .split('|')
        .any(|marker| line.contains(marker))
        && ["handler", "missing-handler", "no handler registered"]
            .into_iter()
            .any(|marker| line.contains(marker))
}
