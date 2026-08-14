pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut state = State::Code;
    text.lines().map(|line| strip(line, &mut state)).collect()
}

enum State {
    Code,
    AwkProgram,
    Heredoc { end: String, strip_tabs: bool },
}

fn strip(line: &str, state: &mut State) -> Option<String> {
    if let State::Heredoc { end, strip_tabs } = state {
        if heredoc_terminates(line, end, *strip_tabs) {
            *state = State::Code;
        }
        return None;
    }
    let visible = strip_awk(line, state)?;
    if let Some((start, token_end, end, strip_tabs)) = heredoc_span(&visible) {
        *state = State::Heredoc { end, strip_tabs };
        return Some(format!("{}{}", &visible[..start], &visible[token_end..]));
    }
    Some(visible)
}

fn strip_awk(line: &str, state: &mut State) -> Option<String> {
    if matches!(state, State::AwkProgram) {
        let index = line.find('\'')?;
        *state = State::Code;
        return strip_awk(&line[index + 1..], state);
    }
    let Some(quote) = awk_program_opener(line) else {
        return Some(line.to_owned());
    };
    let prefix = &line[..quote];
    let tail = &line[quote + 1..];
    if let Some(close) = tail.find('\'') {
        return Some(format!("{prefix}{}", &tail[close + 1..]));
    }
    *state = State::AwkProgram;
    Some(prefix.to_owned())
}

fn heredoc_span(line: &str) -> Option<(usize, usize, String, bool)> {
    let (start, operator_end, strip_tabs) = heredoc_operator(line)?;
    let after_operator = &line[operator_end..];
    let leading = after_operator.len() - after_operator.trim_start().len();
    let (word_len, end) = heredoc_word(after_operator.trim_start())?;
    Some((start, operator_end + leading + word_len, end, strip_tabs))
}

fn heredoc_terminates(line: &str, end: &str, strip_tabs: bool) -> bool {
    if strip_tabs {
        line.trim_start_matches('\t') == end
    } else {
        line == end
    }
}

fn heredoc_operator(line: &str) -> Option<(usize, usize, bool)> {
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < line.len() {
        let tail = &line[index..];
        let character = tail.chars().next()?;
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
            index += character.len_utf8()
                + tail[character.len_utf8()..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8);
            continue;
        } else if tail.starts_with("$((") {
            index += arithmetic_end(tail)?;
            continue;
        } else if character == '#' && comment_start(line, index) {
            return None;
        } else if tail.starts_with("<<<") {
            index += 3;
            continue;
        } else if tail.starts_with("<<") {
            let strip_tabs = tail[2..].starts_with('-');
            return Some((index, index + 2 + strip_tabs as usize, strip_tabs));
        }
        index += character.len_utf8();
    }
    None
}

fn heredoc_word(text: &str) -> Option<(usize, String)> {
    let mut quote = None;
    let mut escaped = false;
    let mut end = String::new();
    for (index, character) in text.char_indices() {
        if escaped {
            end.push(character);
            escaped = false;
        } else if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            } else {
                end.push(character);
            }
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character.is_whitespace() || matches!(character, ';' | '&' | '|' | '<' | '>') {
            return (quote.is_none() && !end.is_empty()).then(|| (index, end));
        } else {
            end.push(character);
        }
    }
    (quote.is_none() && !escaped && !end.is_empty()).then(|| (text.len(), end))
}

fn arithmetic_end(text: &str) -> Option<usize> {
    let mut depth = 1;
    let mut index = 3;
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

fn comment_start(line: &str, index: usize) -> bool {
    line[..index].chars().next_back().is_none_or(|character| {
        character.is_whitespace() || matches!(character, ';' | '&' | '|' | '(' | ')')
    })
}

fn awk_program_opener(line: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(relative) = line[offset..].find("awk") {
        let index = offset + relative;
        let before = line[..index].chars().next_back();
        let after = line[index + 3..].chars().next();
        if !before.is_some_and(is_shell_word) && !after.is_some_and(is_shell_word) {
            if let Some(quote) = program_quote_after_awk(&line[index + 3..]) {
                return Some(index + 3 + quote);
            }
        }
        offset = index + 3;
    }
    None
}

fn program_quote_after_awk(text: &str) -> Option<usize> {
    let mut cursor = 0;
    loop {
        cursor += whitespace_len(&text[cursor..]);
        let token = shell_word_end(&text[cursor..])?;
        let word = &text[cursor..cursor + token];
        if word == "-v" {
            cursor += token + whitespace_len(&text[cursor + token..]);
            cursor += shell_word_end(&text[cursor..])?;
        } else if word.starts_with('-') {
            cursor += token;
        } else {
            return text[cursor..].starts_with('\'').then_some(cursor);
        }
    }
}

fn whitespace_len(text: &str) -> usize {
    text.chars()
        .take_while(|character| character.is_whitespace())
        .map(char::len_utf8)
        .sum()
}

fn shell_word_end(text: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in text.char_indices() {
        quote = match quote {
            Some(delimiter) if character == delimiter => None,
            Some(delimiter) => Some(delimiter),
            None if matches!(character, '\'' | '"') => Some(character),
            None if character.is_whitespace() => return Some(index),
            None => None,
        };
    }
    (!text.is_empty()).then_some(text.len())
}

fn is_shell_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}
