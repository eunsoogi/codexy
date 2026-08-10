pub(super) fn boundaries(instruction: &str) -> Vec<String> {
    let bytes = instruction.as_bytes();
    let (mut start, mut cursor, mut clauses) = (0, 0, Vec::new());
    while cursor < bytes.len() {
        let boundary = match bytes[cursor] {
            b';' => Some((cursor, cursor + 1)),
            _ if cursor > start && simple_task_modal_at(bytes, cursor) => Some((cursor, cursor)),
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

fn simple_task_modal_at(bytes: &[u8], start: usize) -> bool {
    let Some(mut cursor) = word_end(bytes, start, b"simple") else {
        return false;
    };
    cursor = whitespace_after(bytes, cursor);
    if !bytes
        .get(cursor..cursor + 4)
        .is_some_and(|task| task.eq_ignore_ascii_case(b"task"))
    {
        return false;
    }
    cursor += 4 + usize::from(bytes.get(cursor + 4) == Some(&b's'));
    cursor = whitespace_after(bytes, cursor);
    [b"must".as_slice(), b"may", b"should", b"can", b"cannot"]
        .iter()
        .any(|modal| word_end(bytes, cursor, modal).is_some())
}

fn whitespace_after(bytes: &[u8], start: usize) -> usize {
    start
        + bytes[start..]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count()
}

fn word_end(bytes: &[u8], start: usize, word: &[u8]) -> Option<usize> {
    let end = start + word.len();
    bytes
        .get(start..end)
        .filter(|candidate| candidate.eq_ignore_ascii_case(word))
        .filter(|_| {
            bytes
                .get(end)
                .is_none_or(|next| !next.is_ascii_alphabetic())
        })
        .map(|_| end)
}
