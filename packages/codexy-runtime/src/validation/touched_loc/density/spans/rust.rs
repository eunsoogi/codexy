pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut state = State::Code;
    text.lines().map(|line| strip(line, &mut state)).collect()
}

enum State {
    Code,
    BlockComment(usize),
    Raw(String),
    String,
}

fn strip(line: &str, state: &mut State) -> Option<String> {
    let mut visible = String::new();
    let mut remainder = line;
    loop {
        match state {
            State::Raw(end) => {
                let index = remainder.find(end.as_str())?;
                remainder = &remainder[index + end.len()..];
                *state = State::Code;
            }
            State::BlockComment(depth) => {
                let (tail, remaining) = skip_comment(remainder, *depth)?;
                remainder = tail;
                *depth = remaining;
                if remaining != 0 {
                    return Some(visible);
                }
                *state = State::Code;
            }
            State::String => {
                if let Some(tail) = skip_string(remainder) {
                    remainder = tail;
                    *state = State::Code;
                } else {
                    return Some(visible);
                }
            }
            State::Code => {
                let Some((index, span)) = next_span(remainder) else {
                    visible.push_str(remainder);
                    return Some(visible);
                };
                visible.push_str(&remainder[..index]);
                match span {
                    Span::LineComment => return Some(visible),
                    Span::BlockComment => {
                        remainder = &remainder[index + 2..];
                        *state = State::BlockComment(1);
                    }
                    Span::String => {
                        remainder = &remainder[index + 1..];
                        *state = State::String;
                    }
                    Span::Character(length) => remainder = &remainder[index + length..],
                    Span::Raw { opener, close } => {
                        remainder = &remainder[index + opener..];
                        if let Some(closing) = remainder.find(&close) {
                            remainder = &remainder[closing + close.len()..];
                        } else {
                            *state = State::Raw(close);
                            return Some(visible);
                        }
                    }
                }
            }
        }
    }
}

fn skip_comment(line: &str, mut depth: usize) -> Option<(&str, usize)> {
    let mut offset = 0;
    while offset < line.len() {
        let tail = &line[offset..];
        if tail.starts_with("/*") {
            depth += 1;
            offset += 2;
        } else if tail.starts_with("*/") {
            depth -= 1;
            offset += 2;
            if depth == 0 {
                return Some((&line[offset..], 0));
            }
        } else {
            offset += tail.chars().next()?.len_utf8();
        }
    }
    Some(("", depth))
}

fn skip_string(line: &str) -> Option<&str> {
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(&line[index + 1..]);
        }
    }
    None
}

enum Span {
    LineComment,
    BlockComment,
    String,
    Character(usize),
    Raw { opener: usize, close: String },
}

fn next_span(line: &str) -> Option<(usize, Span)> {
    let bytes = line.as_bytes();
    for index in 0..bytes.len() {
        let tail = &bytes[index..];
        if tail.starts_with(b"//") {
            return Some((index, Span::LineComment));
        }
        if tail.starts_with(b"/*") {
            return Some((index, Span::BlockComment));
        }
        if bytes[index] == b'"' {
            return Some((index, Span::String));
        }
        if bytes[index] == b'\'' {
            if let Some(length) = character_len(&line[index..]) {
                return Some((index, Span::Character(length)));
            }
        }
        if bytes[index] == b'r' {
            let hashes = bytes[index + 1..]
                .iter()
                .take_while(|byte| **byte == b'#')
                .count();
            if bytes.get(index + hashes + 1) == Some(&b'"') {
                return Some((
                    index,
                    Span::Raw {
                        opener: hashes + 2,
                        close: format!("\"{}", "#".repeat(hashes)),
                    },
                ));
            }
        }
    }
    None
}

fn character_len(text: &str) -> Option<usize> {
    let mut characters = text.char_indices();
    characters.next()?;
    let (_, first) = characters.next()?;
    let end = if first == '\\' {
        let (index, _) = characters.next()?;
        index + 1
    } else {
        first.len_utf8() + 1
    };
    text[end..].starts_with('\'').then_some(end + 1)
}
