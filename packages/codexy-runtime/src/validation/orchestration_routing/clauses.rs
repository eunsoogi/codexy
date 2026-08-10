pub(super) fn boundaries(instruction: &str) -> Vec<String> {
    let bytes = instruction.as_bytes();
    let (mut start, mut cursor, mut clauses) = (0, 0, Vec::new());
    while cursor < bytes.len() {
        let boundary = match bytes[cursor] {
            b';' => Some((cursor, cursor + 1)),
            b'.' => simple_after(bytes, cursor + 1).map(|next| (cursor, next)),
            _ if word_at(bytes, cursor, b"and")
                && (cursor == 0 || !bytes[cursor - 1].is_ascii_alphanumeric()) =>
            {
                simple_after(bytes, cursor + 3).map(|next| (cursor, next))
            }
            _ => None,
        };
        if let Some((end, next)) = boundary {
            clauses.push(instruction[start..end].to_owned());
            start = next;
            cursor = next;
        } else {
            cursor += 1;
        }
    }
    clauses.push(instruction[start..].to_owned());
    clauses
}

fn simple_after(bytes: &[u8], cursor: usize) -> Option<usize> {
    let next = cursor
        + bytes[cursor..]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
    word_at(bytes, next, b"simple").then_some(next)
}

fn word_at(bytes: &[u8], start: usize, word: &[u8]) -> bool {
    bytes
        .get(start..start + word.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(word))
}
