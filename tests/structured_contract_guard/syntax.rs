const ASSERTION_MACROS: [&str; 6] = [
    "debug_assert_eq",
    "debug_assert_ne",
    "debug_assert",
    "assert_eq",
    "assert_ne",
    "assert",
];
const CONTAINS_MEMBER_NAMES: [&str; 2] = ["r#contains", "contains"];

pub(super) fn assertions(source: &str) -> Vec<(usize, usize, &str)> {
    let mut found = Vec::new();
    let mut offset = 0;
    while let Some(relative) = source[offset..].find("assert") {
        let assert_start = offset + relative;
        let Some((macro_start, open)) = assertion_open(source, assert_start) else {
            offset = assert_start + "assert".len();
            continue;
        };
        if let Some(close) = matching_delimiter(source, open) {
            found.push((macro_start, close, &source[open + 1..close]));
            offset = close + 1;
        } else {
            break;
        }
    }
    found
}

pub(super) fn has_contains_call(text: &str) -> bool {
    let compact: String = text.chars().filter(|character| !character.is_whitespace()).collect();
    let mut tail = compact.as_str();
    while let Some(index) = tail.find('.') {
        let after_dot = &tail[index + 1..];
        if contains_member_call(after_dot) {
            return true;
        }
        tail = after_dot;
    }
    false
}

fn contains_member_call(text: &str) -> bool {
    let Some(after_name) = CONTAINS_MEMBER_NAMES
        .into_iter()
        .find_map(|name| text.strip_prefix(name))
    else {
        return false;
    };
    after_name.starts_with('(') || turbofish_call(after_name)
}

fn assertion_open(source: &str, assert_start: usize) -> Option<(usize, usize)> {
    let start = source[..assert_start]
        .strip_suffix("debug_")
        .map_or(assert_start, str::len);
    if source[..start]
        .chars()
        .next_back()
        .is_some_and(is_identifier_character)
    {
        return None;
    }
    let tail = &source[start..];
    let name = ASSERTION_MACROS.into_iter().find(|name| {
        tail.strip_prefix(name)
            .is_some_and(|after| !after.chars().next().is_some_and(is_identifier_character))
    })?;
    let after_name = tail[name.len()..].trim_start();
    let after_bang = after_name.strip_prefix('!')?.trim_start();
    let delimiter = after_bang.chars().next()?;
    matches!(delimiter, '(' | '{' | '[').then_some((start, source.len() - after_bang.len()))
}

fn turbofish_call(after: &str) -> bool {
    let Some(tail) = after.strip_prefix("::<") else {
        return false;
    };
    let mut depth = 1;
    for (index, character) in tail.char_indices() {
        match character {
            '<' => depth += 1,
            '>' if !tail[..index].ends_with('-') => {
                depth -= 1;
                if depth == 0 {
                    return tail[index + 1..].starts_with('(');
                }
            }
            _ => {}
        }
    }
    false
}

fn matching_delimiter(source: &str, open: usize) -> Option<usize> {
    let mut closes = Vec::new();
    for (relative, character) in source[open..].char_indices() {
        match character {
            '(' => closes.push(')'),
            '{' => closes.push('}'),
            '[' => closes.push(']'),
            ')' | '}' | ']' if closes.last() == Some(&character) => {
                closes.pop();
                if closes.is_empty() {
                    return Some(open + relative);
                }
            }
            ')' | '}' | ']' => return None,
            _ => {}
        }
    }
    None
}

fn is_identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
