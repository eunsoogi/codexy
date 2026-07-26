pub(super) fn assertion_identity(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut identity = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let end = match bytes[index] {
            b'"' => quoted_end(bytes, index, b'"'),
            b'\'' => character_end(bytes, index),
            b'r' => raw_string_end(bytes, index),
            _ => None,
        };
        if let Some(end) = end {
            identity.extend_from_slice(&bytes[index..end]);
            index = end;
        } else {
            if !bytes[index].is_ascii_whitespace() {
                identity.push(bytes[index]);
            }
            index += 1;
        }
    }
    String::from_utf8(identity).expect("assertion identity preserves UTF-8")
}

fn quoted_end(bytes: &[u8], start: usize, quote: u8) -> Option<usize> {
    let mut cursor = start + 1;
    let mut escaped = false;
    while cursor < bytes.len() {
        if !escaped && bytes[cursor] == quote {
            return Some(cursor + 1);
        }
        escaped = !escaped && bytes[cursor] == b'\\';
        if bytes[cursor] != b'\\' {
            escaped = false;
        }
        cursor += 1;
    }
    None
}

fn character_end(bytes: &[u8], start: usize) -> Option<usize> {
    let content = start + 1;
    let close = if bytes.get(content) == Some(&b'\\') {
        bytes[content + 1..]
            .iter()
            .take(10)
            .position(|byte| *byte == b'\'')
            .map(|offset| content + offset + 1)
    } else {
        std::str::from_utf8(bytes.get(content..)?)
            .ok()
            .and_then(|tail| {
                tail.chars()
                    .next()
                    .map(|character| content + character.len_utf8())
                    .filter(|close| bytes.get(*close) == Some(&b'\''))
            })
    }?;
    Some(close + 1)
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'r') {
        return None;
    }
    let mut cursor = start + 1;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    let hashes = cursor - start - 1;
    cursor += 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes.get(cursor + 1..cursor + 1 + hashes) == Some(&vec![b'#'; hashes])
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    None
}
