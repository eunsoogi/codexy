#[derive(Default)]
pub(super) struct ScopeTracker {
    depth: usize,
    block_comments: usize,
    quoted: Option<u8>,
    raw_hashes: Option<usize>,
}

impl ScopeTracker {
    pub(super) fn is_outer(&self) -> bool {
        self.is_outer_scope() && self.block_comments == 0
    }

    pub(super) fn is_outer_scope(&self) -> bool {
        self.depth == 0 && self.quoted.is_none() && self.raw_hashes.is_none()
    }

    pub(super) fn observe_with_outer_remainder<'a>(&mut self, line: &'a str) -> Option<&'a str> {
        let started_outer = self.is_outer();
        let bytes = line.as_bytes();
        let mut index = 0;
        let mut outer_offset = None;
        while index < bytes.len() {
            if !started_outer && outer_offset.is_none() && self.is_outer() {
                outer_offset = Some(index);
            }
            if self.block_comments > 0 {
                if pair(bytes, index, b'/', b'*') {
                    self.block_comments += 1;
                    index += 2;
                } else if pair(bytes, index, b'*', b'/') {
                    self.block_comments -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
                continue;
            }
            if let Some(hashes) = self.raw_hashes {
                if bytes[index] == b'"' && closes_raw_string(bytes, index, hashes) {
                    self.raw_hashes = None;
                    index += hashes + 1;
                } else {
                    index += 1;
                }
                continue;
            }
            if let Some(delimiter) = self.quoted {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else {
                    if bytes[index] == delimiter {
                        self.quoted = None;
                    }
                    index += 1;
                }
                continue;
            }
            if pair(bytes, index, b'/', b'/') {
                break;
            }
            if pair(bytes, index, b'/', b'*') {
                self.block_comments += 1;
                index += 2;
                continue;
            }
            if let Some((hashes, next)) = raw_string_start(bytes, index) {
                self.raw_hashes = Some(hashes);
                index = next;
                continue;
            }
            if let Some((delimiter, next)) = quoted_start(bytes, index) {
                self.quoted = Some(delimiter);
                index = next;
                continue;
            }
            match bytes[index] {
                b'{' | b'(' | b'[' => self.depth += 1,
                b'}' | b')' | b']' => self.depth = self.depth.saturating_sub(1),
                _ => {}
            }
            index += 1;
        }
        if !started_outer && outer_offset.is_none() && self.is_outer() {
            outer_offset = Some(bytes.len());
        }
        outer_offset.map(|offset| &line[offset..])
    }
}

pub(super) fn outer_attribute_remainder(source: &str) -> Option<&str> {
    let source = source.strip_prefix('#')?.trim_start().strip_prefix('[')?;
    let mut scope = ScopeTracker {
        depth: 1,
        ..Default::default()
    };
    scope.observe_with_outer_remainder(source)
}

fn pair(bytes: &[u8], index: usize, first: u8, second: u8) -> bool {
    bytes.get(index) == Some(&first) && bytes.get(index + 1) == Some(&second)
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

fn closes_raw_string(bytes: &[u8], quote: usize, hashes: usize) -> bool {
    bytes
        .get(quote + 1..quote + 1 + hashes)
        .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
}

fn quoted_start(bytes: &[u8], index: usize) -> Option<(u8, usize)> {
    match (bytes.get(index), bytes.get(index + 1)) {
        (Some(b'"'), _) => Some((b'"', index + 1)),
        (Some(b'b' | b'c'), Some(b'"')) => Some((b'"', index + 2)),
        (Some(b'\''), _) if has_closing_quote(bytes, index + 1) => Some((b'\'', index + 1)),
        (Some(b'b'), Some(b'\'')) if has_closing_quote(bytes, index + 2) => {
            Some((b'\'', index + 2))
        }
        _ => None,
    }
}

fn has_closing_quote(bytes: &[u8], mut index: usize) -> bool {
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
        } else if bytes[index] == b'\'' {
            return true;
        } else {
            index += 1;
        }
    }
    false
}
