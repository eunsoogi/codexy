pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut state = State::Code;
    text.lines().map(|line| strip(line, &mut state)).collect()
}

enum State {
    Code,
    AwkProgram,
    Heredoc(String),
}

fn strip(line: &str, state: &mut State) -> Option<String> {
    if let State::Heredoc(end) = state {
        if line.trim() == end {
            *state = State::Code;
        }
        return None;
    }
    let visible = strip_awk(line, state)?;
    if let Some((start, token_end, end)) = heredoc_span(&visible) {
        *state = State::Heredoc(end);
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

fn heredoc_span(line: &str) -> Option<(usize, usize, String)> {
    let start = unquoted_heredoc_start(line)?;
    let after_operator = &line[start + 2..];
    let dash = after_operator.starts_with('-') as usize;
    let after_operator = &after_operator[dash..];
    let leading = after_operator.len() - after_operator.trim_start().len();
    let token = after_operator
        .trim_start()
        .split(|character: char| character.is_whitespace() || matches!(character, ';' | '&' | '|'))
        .next()?;
    let end = token.trim_matches(|character| matches!(character, '\'' | '"'));
    (!end.is_empty()).then(|| {
        (
            start,
            start + 2 + dash + leading + token.len(),
            end.to_owned(),
        )
    })
}

fn unquoted_heredoc_start(line: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in line.char_indices() {
        quote = match quote {
            Some(delimiter) if character == delimiter => None,
            Some(delimiter) => Some(delimiter),
            None if matches!(character, '\'' | '"') => Some(character),
            None if line[index..].starts_with("<<") => return Some(index),
            None => None,
        };
    }
    None
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
