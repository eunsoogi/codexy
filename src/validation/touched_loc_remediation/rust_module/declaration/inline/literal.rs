pub(super) fn skip(bytes: &[u8], index: usize) -> Option<Option<usize>> {
    if let Some((hashes, content)) = raw_string_start(bytes, index) {
        return Some(Some(skip_raw_string(bytes, content, hashes)?));
    }
    if let Some(content) = string_start(bytes, index) {
        return Some(Some(skip_string(bytes, content)?));
    }
    char_literal_end(bytes, index)
}

fn raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    let mut cursor = index;
    if matches!(bytes.get(cursor), Some(b'b' | b'c')) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }
    cursor += 1;
    let hash_start = cursor;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor - hash_start, cursor + 1))
}

fn skip_raw_string(bytes: &[u8], mut index: usize, hashes: usize) -> Option<usize> {
    while index < bytes.len() {
        if bytes[index] == b'"'
            && bytes
                .get(index + 1..index + 1 + hashes)
                .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
        {
            return Some(index + hashes + 1);
        }
        index += 1;
    }
    None
}

fn string_start(bytes: &[u8], index: usize) -> Option<usize> {
    match (bytes.get(index), bytes.get(index + 1)) {
        (Some(b'"'), _) => Some(index + 1),
        (Some(b'b' | b'c'), Some(b'"')) => Some(index + 2),
        _ => None,
    }
}

fn skip_string(bytes: &[u8], mut index: usize) -> Option<usize> {
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index = index.checked_add(2)?;
        } else if bytes[index] == b'"' {
            return Some(index + 1);
        } else {
            index += 1;
        }
    }
    None
}

fn char_literal_end(bytes: &[u8], index: usize) -> Option<Option<usize>> {
    let (content, byte_literal) = match (bytes.get(index), bytes.get(index + 1)) {
        (Some(b'\''), _) => (index + 1, false),
        (Some(b'b'), Some(b'\'')) => (index + 2, true),
        _ => return Some(None),
    };
    let Some(end) = char_content_end(bytes, content, byte_literal) else {
        return if byte_literal { None } else { Some(None) };
    };
    if bytes.get(end) == Some(&b'\'') {
        Some(Some(end + 1))
    } else if byte_literal || bytes.get(content) == Some(&b'\\') {
        None
    } else {
        Some(None)
    }
}

fn char_content_end(bytes: &[u8], content: usize, byte_literal: bool) -> Option<usize> {
    if bytes.get(content) == Some(&b'\\') {
        return escape_end(bytes, content, byte_literal);
    }
    let character = std::str::from_utf8(bytes.get(content..)?)
        .ok()?
        .chars()
        .next()?;
    (!matches!(character, '\'' | '\r' | '\n') && (!byte_literal || character.is_ascii()))
        .then_some(content + character.len_utf8())
}

fn escape_end(bytes: &[u8], slash: usize, byte_literal: bool) -> Option<usize> {
    match bytes.get(slash + 1)? {
        b'\\' | b'\'' | b'"' | b'n' | b'r' | b't' | b'0' => Some(slash + 2),
        b'x' if bytes
            .get(slash + 2..slash + 4)
            .is_some_and(|digits| digits.iter().all(u8::is_ascii_hexdigit)) =>
        {
            Some(slash + 4)
        }
        b'u' if !byte_literal => unicode_escape_end(bytes, slash),
        _ => None,
    }
}

fn unicode_escape_end(bytes: &[u8], slash: usize) -> Option<usize> {
    (bytes.get(slash + 2) == Some(&b'{')).then_some(())?;
    let close = bytes[slash + 3..].iter().position(|byte| *byte == b'}')? + slash + 3;
    let digits = bytes.get(slash + 3..close)?;
    let count = digits.iter().filter(|byte| **byte != b'_').count();
    (!digits.is_empty()
        && (1..=6).contains(&count)
        && digits
            .iter()
            .all(|byte| *byte == b'_' || byte.is_ascii_hexdigit()))
    .then_some(close + 1)
}
