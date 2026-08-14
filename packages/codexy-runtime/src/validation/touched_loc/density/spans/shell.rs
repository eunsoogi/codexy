use std::collections::VecDeque;

use super::shell_heredoc::{Heredoc, Span, spans};

pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut state = State::Code;
    text.lines().map(|line| strip(line, &mut state)).collect()
}

enum State {
    Code,
    AwkProgram,
    Heredocs(VecDeque<Heredoc>),
}

fn strip(line: &str, state: &mut State) -> Option<String> {
    if let State::Heredocs(pending) = state {
        if pending
            .front()
            .is_some_and(|heredoc| heredoc.terminates(line))
        {
            pending.pop_front();
        }
        if pending.is_empty() {
            *state = State::Code;
        }
        return None;
    }
    let visible = strip_awk(line, state)?;
    let heredocs = spans(&visible);
    if heredocs.is_empty() {
        return Some(visible);
    }
    let mut source = visible;
    for span in heredocs.iter().rev() {
        source.replace_range(span.start..span.end, "");
    }
    *state = State::Heredocs(heredocs.into_iter().map(Span::into_heredoc).collect());
    Some(source)
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
