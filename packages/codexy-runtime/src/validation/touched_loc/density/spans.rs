use std::path::Path;

mod rust;

pub(super) fn visible_lines(path: &Path, text: &str) -> Vec<Option<String>> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => rust::lines(text),
        Some("sh") => awk_lines(text),
        None if is_shell_script(path, text) => awk_lines(text),
        Some("md") => markdown_lines(text),
        _ => text.lines().map(|line| Some(line.to_owned())).collect(),
    }
}

fn is_shell_script(path: &Path, text: &str) -> bool {
    path.starts_with("scripts")
        && text
            .lines()
            .next()
            .is_some_and(|line| line.starts_with("#!") && line.contains("sh"))
}

fn awk_lines(text: &str) -> Vec<Option<String>> {
    let mut quoted = false;
    text.lines()
        .map(|line| strip_awk(line, &mut quoted))
        .collect()
}

fn strip_awk(line: &str, quoted: &mut bool) -> Option<String> {
    if *quoted {
        let index = line.find('\'')?;
        *quoted = false;
        return strip_awk(&line[index + 1..], quoted);
    }
    let Some(quote) = awk_program_opener(line) else {
        return Some(line.to_owned());
    };
    let prefix = &line[..quote];
    let tail = &line[quote + 1..];
    if let Some(close) = tail.find('\'') {
        return Some(format!("{prefix}{}", &tail[close + 1..]));
    }
    *quoted = true;
    Some(prefix.to_owned())
}

fn awk_program_opener(line: &str) -> Option<usize> {
    let awk = line.find("awk")?;
    let before = line[..awk].chars().next_back();
    let after = line[awk + 3..].chars().next();
    if before.is_some_and(is_shell_word) || after.is_some_and(is_shell_word) {
        return None;
    }
    let mut cursor = awk + 3;
    loop {
        cursor += whitespace_len(&line[cursor..]);
        let token = shell_word_end(&line[cursor..])?;
        let word = &line[cursor..cursor + token];
        if word == "-v" {
            cursor += token + whitespace_len(&line[cursor + token..]);
            cursor += shell_word_end(&line[cursor..])?;
        } else if word.starts_with('-') {
            cursor += token;
        } else {
            return line[cursor..].starts_with('\'').then_some(cursor);
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

fn markdown_lines(text: &str) -> Vec<Option<String>> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut visible = Vec::with_capacity(lines.len());
    let mut fence = None;
    for line in &lines {
        if let Some((marker, count)) = fence_run(line) {
            if fence.is_some_and(|(active, minimum)| active == marker && count >= minimum) {
                fence = None;
            } else if fence.is_none() {
                fence = Some((marker, count));
            }
            visible.push(None);
        } else if fence.is_some() {
            visible.push(None);
        } else {
            visible.push(Some((*line).to_owned()));
        }
    }
    for index in 1..lines.len() {
        if visible[index].is_some() && table_delimiter(lines[index]) && table_row(lines[index - 1])
        {
            visible[index - 1] = None;
            visible[index] = None;
            for following in index + 1..lines.len() {
                if !table_row(lines[following]) {
                    break;
                }
                visible[following] = None;
            }
        }
    }
    visible
}

fn fence_run(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    let marker = trimmed
        .chars()
        .next()
        .filter(|character| matches!(character, '`' | '~'))?;
    let count = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (count >= 3).then_some((marker, count))
}

fn table_row(line: &str) -> bool {
    line.matches('|').count() >= 1
}

fn table_delimiter(line: &str) -> bool {
    let cells = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>();
    cells.len() >= 2
        && cells.into_iter().all(|cell| {
            let core = cell.trim_matches(':');
            core.len() >= 3 && core.chars().all(|character| character == '-')
        })
}
