pub(super) fn sanitize(source: &str) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            index = blank_until(&mut bytes, index, b"\n");
        } else if bytes[index..].starts_with(b"/*") {
            index = blank_block_comment(&mut bytes, index);
        } else if let Some((hashes, content)) = raw_string_start(&bytes, index) {
            index = blank_raw_string(&mut bytes, index, hashes, content);
        } else if bytes[index] == b'"' {
            index = blank_string(&mut bytes, index);
        } else if bytes[index] == b'\'' {
            index = blank_character(&mut bytes, index).unwrap_or(index + 1);
        } else {
            index += 1;
        }
    }
    String::from_utf8(bytes).expect("sanitizing UTF-8 preserves valid bytes")
}

pub(super) fn strip_comments(source: &str) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            index = blank_until(&mut bytes, index, b"\n");
        } else if bytes[index..].starts_with(b"/*") {
            index = blank_block_comment(&mut bytes, index);
        } else if let Some((hashes, content)) = raw_string_start(&bytes, index) {
            index = skip_raw_string(&bytes, hashes, content);
        } else if bytes[index] == b'"' {
            index = skip_string(&bytes, index);
        } else if bytes[index] == b'\'' {
            index = skip_character(&bytes, index).unwrap_or(index + 1);
        } else {
            index += 1;
        }
    }
    String::from_utf8(bytes).expect("stripping comments preserves valid UTF-8")
}

fn skip_raw_string(bytes: &[u8], hashes: usize, mut cursor: usize) -> usize {
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes.get(cursor + 1..cursor + 1 + hashes) == Some(&vec![b'#'; hashes])
        {
            return cursor + 1 + hashes;
        }
        cursor += 1;
    }
    bytes.len()
}

fn skip_string(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start + 1;
    let mut escaped = false;
    while cursor < bytes.len() {
        if !escaped && bytes[cursor] == b'"' {
            return cursor + 1;
        }
        escaped = !escaped && bytes[cursor] == b'\\';
        if bytes[cursor] != b'\\' {
            escaped = false;
        }
        cursor += 1;
    }
    cursor
}

fn skip_character(bytes: &[u8], start: usize) -> Option<usize> {
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

fn blank_character(bytes: &mut [u8], start: usize) -> Option<usize> {
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
    blank_range(bytes, start, close + 1);
    Some(close + 1)
}

fn raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if bytes.get(index) != Some(&b'r') {
        return None;
    }
    let mut cursor = index + 1;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor - index - 1, cursor + 1))
}

fn blank_raw_string(bytes: &mut [u8], start: usize, hashes: usize, mut cursor: usize) -> usize {
    let mut end = bytes.len();
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes.get(cursor + 1..cursor + 1 + hashes) == Some(&vec![b'#'; hashes])
        {
            end = cursor + 1 + hashes;
            break;
        }
        cursor += 1;
    }
    blank_range(bytes, start, end);
    end
}

fn blank_string(bytes: &mut [u8], start: usize) -> usize {
    let mut cursor = start + 1;
    let mut escaped = false;
    while cursor < bytes.len() {
        if !escaped && bytes[cursor] == b'"' {
            cursor += 1;
            break;
        }
        escaped = !escaped && bytes[cursor] == b'\\';
        if bytes[cursor] != b'\\' {
            escaped = false;
        }
        cursor += 1;
    }
    blank_range(bytes, start, cursor);
    cursor
}

fn blank_block_comment(bytes: &mut [u8], start: usize) -> usize {
    let mut cursor = start + 2;
    let mut depth = 1;
    while cursor < bytes.len() && depth > 0 {
        if bytes[cursor..].starts_with(b"/*") {
            depth += 1;
            cursor += 2;
        } else if bytes[cursor..].starts_with(b"*/") {
            depth -= 1;
            cursor += 2;
        } else {
            cursor += 1;
        }
    }
    blank_range(bytes, start, cursor);
    cursor
}

fn blank_until(bytes: &mut [u8], start: usize, marker: &[u8]) -> usize {
    let end = bytes[start..]
        .windows(marker.len())
        .position(|window| window == marker)
        .map_or(bytes.len(), |offset| start + offset);
    blank_range(bytes, start, end);
    end
}

fn blank_range(bytes: &mut [u8], start: usize, end: usize) {
    for byte in &mut bytes[start..end] {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
}
