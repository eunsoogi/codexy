pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut state = State::Code;
    text.lines().map(|line| strip(line, &mut state)).collect()
}

enum State {
    Code,
    BlockComment,
    String(char),
    Regex,
}

fn strip(line: &str, state: &mut State) -> Option<String> {
    let mut visible = String::new();
    let mut remainder = line;
    loop {
        match state {
            State::BlockComment => match remainder.find("*/") {
                Some(index) => {
                    remainder = &remainder[index + 2..];
                    *state = State::Code;
                }
                None => return Some(visible),
            },
            State::String(delimiter) => match skip_string(remainder, *delimiter) {
                Some(tail) => {
                    remainder = tail;
                    *state = State::Code;
                }
                None => return Some(visible),
            },
            State::Regex => match skip_regex(remainder) {
                Some(tail) => {
                    remainder = tail;
                    *state = State::Code;
                }
                None => return Some(visible),
            },
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
                        *state = State::BlockComment;
                    }
                    Span::String(delimiter) => {
                        remainder = &remainder[index + delimiter.len_utf8()..];
                        *state = State::String(delimiter);
                    }
                    Span::Regex => {
                        remainder = &remainder[index + 1..];
                        *state = State::Regex;
                    }
                }
            }
        }
    }
}

fn skip_regex(line: &str) -> Option<&str> {
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '/' {
            return Some(&line[index + 1..]);
        }
    }
    None
}

fn skip_string(line: &str, delimiter: char) -> Option<&str> {
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == delimiter {
            return Some(&line[index + delimiter.len_utf8()..]);
        }
    }
    None
}

enum Span {
    LineComment,
    BlockComment,
    String(char),
    Regex,
}

fn next_span(line: &str) -> Option<(usize, Span)> {
    for (index, character) in line.char_indices() {
        let tail = &line[index..];
        if tail.starts_with("//") {
            return Some((index, Span::LineComment));
        }
        if tail.starts_with("/*") {
            return Some((index, Span::BlockComment));
        }
        if character == '/' && regex_context(&line[..index]) {
            return Some((index, Span::Regex));
        }
        if matches!(character, '\'' | '"' | '`') {
            return Some((index, Span::String(character)));
        }
    }
    None
}

fn regex_context(prefix: &str) -> bool {
    prefix
        .trim_end()
        .chars()
        .next_back()
        .is_none_or(|character| {
            matches!(
                character,
                '=' | '(' | '[' | '{' | ',' | ':' | ';' | '!' | '&' | '|' | '?'
            )
        })
}
