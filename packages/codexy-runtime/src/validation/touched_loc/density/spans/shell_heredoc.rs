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

pub(super) fn spans(line: &str, code: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut index = 0;
    while index < code.len() {
        let tail = &code[index..];
        if tail.starts_with("<<<") {
            index += 3;
        } else if tail.starts_with("<<") {
            let strip_tabs = line[index + 2..].starts_with('-');
            let operator_end = index + 2 + strip_tabs as usize;
            if let Some((word_len, end)) = word(&line[operator_end..]) {
                spans.push(Span {
                    start: index,
                    end: operator_end + word_len,
                    heredoc: Heredoc { end, strip_tabs },
                });
                index = operator_end + word_len;
            } else {
                index += 2;
            }
        } else {
            index += tail
                .chars()
                .next()
                .expect("index must be in bounds")
                .len_utf8();
        }
    }
    spans
}

fn word(text: &str) -> Option<(usize, String)> {
    let leading = text.len() - text.trim_start().len();
    let text = &text[leading..];
    let mut quote = None;
    let mut end = String::new();
    let mut index = 0;
    while index < text.len() {
        let tail = &text[index..];
        let character = tail.chars().next().expect("index must be in bounds");
        if let Some(delimiter) = quote {
            if character == '\\' && delimiter == '"' {
                let next = tail[1..].chars().next()?;
                if matches!(next, '$' | '"' | '\\') || next == char::from(96) {
                    end.push(next);
                    index += 1 + next.len_utf8();
                    continue;
                }
                if next == '\n' {
                    index += 2;
                    continue;
                }
                end.push(character);
            } else if character == delimiter {
                quote = None;
            } else {
                end.push(character);
            }
        } else if character == '\\' {
            let next = tail[1..].chars().next()?;
            end.push(next);
            index += 1 + next.len_utf8();
            continue;
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if boundary(character) {
            return (quote.is_none() && !end.is_empty()).then(|| (leading + index, end));
        } else {
            end.push(character);
        }
        index += character.len_utf8();
    }
    (quote.is_none() && !end.is_empty()).then(|| (text.len() + leading, end))
}

fn boundary(character: char) -> bool {
    character.is_whitespace() || matches!(character, ';' | '&' | '|' | '<' | '>')
}
