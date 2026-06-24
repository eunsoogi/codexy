pub(super) fn line_is_list_item(line: &str) -> bool {
    matches!(line.trim_start().as_bytes(), [b'-' | b'*' | b'+', b' ', ..])
}

pub(super) fn strip_list_marker(value: &str) -> &str {
    let value = value.trim_start();
    value
        .strip_prefix(['-', '*', '+'])
        .unwrap_or(value)
        .trim_start()
}

pub(super) fn key_allows_list_metadata_boundary(key: &str) -> bool {
    key.chars().any(|character| character.is_ascii_alphabetic())
        && key.chars().all(|character| {
            character.is_ascii_alphabetic()
                || character.is_ascii_whitespace()
                || matches!(character, '-' | '/')
        })
}

pub(super) fn has_helper_only_purpose(value: &str) -> bool {
    if !value.contains("used only for") || value.contains("review-response") {
        return false;
    }
    [
        "helper",
        "qa",
        "QA",
        "reviewer",
        "reviewer gate",
        "research",
        "verification",
        "test",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}

pub(super) fn has_unavailable_helper_rationale(value: &str) -> bool {
    "subagent unavailable|sub-agent unavailable|multi_agent unavailable|multi-agent unavailable|subagent tools unavailable|sub-agent tools unavailable|multi_agent tools unavailable|multi-agent tools unavailable|spawn_agent unavailable"
        .split('|')
        .any(|marker| value.contains(marker))
}
