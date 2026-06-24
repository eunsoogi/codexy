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
