pub(super) fn field_value<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    line.split_once(':')
        .and_then(|(key, value)| metadata_key(key).contains(field).then_some(value.trim()))
}

pub(super) fn metadata_key(key: &str) -> &str {
    let key = key.trim().trim_start_matches(['-', '*']).trim_start();
    key.strip_prefix("[x]")
        .or_else(|| key.strip_prefix("[X]"))
        .unwrap_or(key)
        .trim_start()
}

fn has_absent_value(value: &str) -> bool {
    matches!(
        trimmed_value(value),
        "no" | "none" | "false" | "missing" | "absent" | "not provided"
    )
}

pub(super) fn has_absent_field_value(value: &str, field: &str) -> bool {
    let value = trimmed_value(value);
    if has_absent_value(value) {
        return true;
    }
    if value
        .strip_prefix("not ")
        .and_then(|suffix| suffix.strip_prefix(field))
        .is_some_and(|suffix| {
            suffix.is_empty()
                || suffix.starts_with(char::is_whitespace)
                || matches!(suffix.chars().next(), Some('.' | ',' | ';' | ':'))
        })
    {
        return true;
    }
    "not provided|without|missing|absent|none|not|no"
        .split('|')
        .any(|marker| {
            let Some(after_marker) = value.strip_prefix(marker) else {
                return false;
            };
            let Some(separator) = after_marker.chars().next() else {
                return true;
            };
            if !separator.is_ascii_whitespace() && !matches!(separator, '.' | ',' | ';' | ':') {
                return false;
            }
            !after_marker.contains(field)
        })
}

pub(super) fn trimmed_value(value: &str) -> &str {
    value.trim_matches(|character: char| {
        character.is_ascii_whitespace() || matches!(character, '.' | ',' | ';')
    })
}
