pub(super) struct Span {
    pub(super) start: usize,
    pub(super) end: usize,
    heredoc: Heredoc,
}

pub(super) struct Heredoc {
    end: String,
    strip_tabs: bool,
}

impl Span {
    pub(super) fn into_heredoc(self) -> Heredoc {
        self.heredoc
    }
}

impl Heredoc {
    pub(super) fn terminates(&self, line: &str) -> bool {
        if self.strip_tabs {
            line.trim_start_matches('\t') == self.end
        } else {
            line == self.end
        }
    }
}

pub(super) fn spans(line: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < line.len() {
        let tail = &line[index..];
        let character = tail.chars().next().expect("index must be in bounds");
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' && delimiter == '"' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            index += character.len_utf8();
            continue;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character == '\\' {
            index += escaped_char_len(tail);
            continue;
        } else if tail.starts_with("$((") || tail.starts_with("((") {
            index += arithmetic_len(tail).unwrap_or(tail.len());
            continue;
        } else if character == '#' && comment_start(line, index) {
            break;
        } else if tail.starts_with("<<<") {
            index += 3;
            continue;
        } else if tail.starts_with("<<") {
            let strip_tabs = tail[2..].starts_with('-');
            let operator_end = index + 2 + strip_tabs as usize;
            if let Some((word_len, end)) = word(&line[operator_end..]) {
                spans.push(Span {
                    start: index,
                    end: operator_end + word_len,
                    heredoc: Heredoc { end, strip_tabs },
                });
                index = operator_end + word_len;
                continue;
            }
        }
        index += character.len_utf8();
    }
    spans
}

fn word(text: &str) -> Option<(usize, String)> {
    let leading = text.len() - text.trim_start().len();
    let text = &text[leading..];
    let mut quote = None;
    let mut escaped = false;
    let mut end = String::new();
    for (index, character) in text.char_indices() {
        if escaped {
            end.push(character);
            escaped = false;
        } else if let Some(delimiter) = quote {
            if character == '\\' && delimiter == '"' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            } else {
                end.push(character);
            }
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if boundary(character) {
            return (quote.is_none() && !end.is_empty()).then(|| (leading + index, end));
        } else {
            end.push(character);
        }
    }
    (quote.is_none() && !escaped && !end.is_empty()).then(|| (text.len() + leading, end))
}

fn arithmetic_len(text: &str) -> Option<usize> {
    let mut depth = 1;
    let mut index = 2 + text.starts_with("$") as usize;
    while index < text.len() {
        let tail = &text[index..];
        if tail.starts_with("((") {
            depth += 1;
            index += 2;
        } else if tail.starts_with("))") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return Some(index);
            }
        } else {
            index += tail.chars().next()?.len_utf8();
        }
    }
    None
}

fn escaped_char_len(text: &str) -> usize {
    let slash = '\\'.len_utf8();
    slash + text[slash..].chars().next().map_or(0, char::len_utf8)
}

fn boundary(character: char) -> bool {
    character.is_whitespace() || matches!(character, ';' | '&' | '|' | '<' | '>')
}

fn comment_start(line: &str, index: usize) -> bool {
    line[..index].chars().next_back().is_none_or(|character| {
        character.is_whitespace() || matches!(character, ';' | '&' | '|' | '(' | ')')
    })
}
