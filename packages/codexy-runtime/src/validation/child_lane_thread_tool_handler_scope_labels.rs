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
    if label.is_empty()
        || matches!(
            label.to_ascii_lowercase().as_str(),
            "owner" | "owners" | "ownership" | "metadata" | "type"
        )
    {
        return key;
    }
    rest[label_end..].trim_start_matches(|ch: char| ch.is_whitespace() || ch == '-' || ch == '.')
}

pub(super) fn is_handler_capture_line(line: &str) -> bool {
    "captured|classified|recorded|reported|routed|tracked"
        .split('|')
        .any(|marker| line.contains(marker))
        && ["handler", "missing-handler", "no handler registered"]
            .into_iter()
            .any(|marker| line.contains(marker))
}
