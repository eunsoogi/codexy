pub(super) fn raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    let mut cursor = index;
    if bytes.get(cursor) == Some(&b'b') {
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
    let hashes = cursor - hash_start;
    if bytes.get(cursor) == Some(&b'"') {
        Some((cursor + 1, hashes))
    } else {
        None
    }
}

pub(super) fn raw_string_end(bytes: &[u8], content_start: usize, hashes: usize) -> usize {
    let mut cursor = content_start;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"' {
            let mut end = cursor + 1;
            let mut matches = true;
            for _ in 0..hashes {
                if bytes.get(end) != Some(&b'#') {
                    matches = false;
                    break;
                }
                end += 1;
            }
            if matches {
                return end;
            }
        }
        cursor += 1;
    }
    bytes.len()
}

pub(super) fn quoted_string_end(bytes: &[u8], opening_quote: usize) -> usize {
    let mut cursor = opening_quote + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor = cursor.saturating_add(2),
            b'"' => return cursor + 1,
            _ => cursor += 1,
        }
    }
    bytes.len()
}
