#[derive(Debug)]
pub(crate) struct Word {
    pub(crate) text: String,
    start: usize,
    end: usize,
}

pub(crate) fn commands(text: &str) -> Vec<Vec<Word>> {
    let mut commands = Vec::new();
    let mut command = Vec::new();
    let mut word: Option<(usize, usize, String)> = None;
    let mut quote = None;
    let mut chars = text.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        if let Some(delimiter) = quote {
            let entry = word.get_or_insert_with(|| (index, index, String::new()));
            entry.1 = index + character.len_utf8();
            if character == delimiter {
                quote = None;
            } else if character == '\\' && delimiter == '"' {
                append_escaped(&mut chars, entry);
            } else {
                entry.2.push(character);
            }
            continue;
        }

        match character {
            '\'' | '"' => {
                let entry = word.get_or_insert_with(|| (index, index, String::new()));
                entry.1 = index + 1;
                quote = Some(character);
            }
            '\\' => {
                if chars.peek().is_some_and(|(_, next)| *next == '\n') {
                    chars.next();
                } else {
                    let entry = word.get_or_insert_with(|| (index, index, String::new()));
                    entry.1 = index + 1;
                    append_escaped(&mut chars, entry);
                }
            }
            '#' if word.is_none() => {
                for (_, next) in chars.by_ref() {
                    if next == '\n' {
                        finish_command(&mut command, &mut commands);
                        break;
                    }
                }
            }
            '\n' => {
                finish_word(&mut word, &mut command);
                finish_command(&mut command, &mut commands);
            }
            ';' | '|' | '&' | '(' | ')' => {
                finish_word(&mut word, &mut command);
                finish_command(&mut command, &mut commands);
            }
            character if character.is_whitespace() => finish_word(&mut word, &mut command),
            _ => {
                let entry = word.get_or_insert_with(|| (index, index, String::new()));
                entry.1 = index + character.len_utf8();
                entry.2.push(character);
            }
        }
    }
    finish_word(&mut word, &mut command);
    finish_command(&mut command, &mut commands);
    commands
}

pub(crate) fn runtime_exec<'a>(text: &'a str, server: &str) -> Option<Vec<Word>> {
    let mut matches = commands(text).into_iter().filter(|command| {
        command.first().is_some_and(|word| word.text == "exec")
            && has_sequence(command, &["codexy-mcp-runtime", server])
    });
    let command = matches.next()?;
    matches.next().is_none().then_some(command)
}

pub(crate) fn has_sequence(command: &[Word], expected: &[&str]) -> bool {
    command.windows(expected.len()).any(|words| {
        words
            .iter()
            .zip(expected)
            .all(|(word, value)| word.text == *value)
    })
}

pub(crate) fn unique_option_value<'a>(command: &'a [Word], option: &str) -> Option<&'a str> {
    let positions = command
        .iter()
        .enumerate()
        .filter(|(_, word)| word.text == option)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    (positions.len() == 1)
        .then(|| command.get(positions[0] + 1))
        .flatten()
        .map(|word| word.text.as_str())
}

pub(crate) fn replace_runtime_pin(
    text: &str,
    server: &str,
    current: &str,
    requested: &str,
) -> Option<String> {
    let command = runtime_exec(text, server)?;
    if unique_option_value(&command, "--from") != Some(current) {
        return None;
    }
    let pin = command
        .windows(2)
        .find(|words| words[0].text == "--from")
        .map(|words| &words[1])?;
    Some(format!(
        "{}\"{requested}\"{}",
        &text[..pin.start],
        &text[pin.end..]
    ))
}

fn append_escaped(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    word: &mut (usize, usize, String),
) {
    if let Some((index, escaped)) = chars.next() {
        word.1 = index + escaped.len_utf8();
        if escaped != '\n' {
            word.2.push(escaped);
        }
    }
}

fn finish_word(word: &mut Option<(usize, usize, String)>, command: &mut Vec<Word>) {
    if let Some((start, end, text)) = word.take() {
        command.push(Word { text, start, end });
    }
}

fn finish_command(command: &mut Vec<Word>, commands: &mut Vec<Vec<Word>>) {
    if !command.is_empty() {
        commands.push(std::mem::take(command));
    }
}

#[cfg(test)]
mod tests {
    use super::{replace_runtime_pin, runtime_exec};

    #[test]
    fn comments_and_continuations_do_not_create_runtime_commands() {
        let text = "true;# decoy\nexec uvx --from \"pkg==2\" codexy-mcp-runtime lsp\n";
        let command = runtime_exec(text, "lsp").expect("one runtime command");
        assert!(command.iter().any(|word| word.text == "pkg==2"));
    }

    #[test]
    fn replaces_only_the_active_pin_word() {
        let text = "# pkg==1\nexec uvx --from \"pkg==1\" codexy-mcp-runtime lsp\n";
        let updated = replace_runtime_pin(text, "lsp", "pkg==1", "pkg==2").unwrap();
        assert_eq!(
            updated,
            "# pkg==1\nexec uvx --from \"pkg==2\" codexy-mcp-runtime lsp\n"
        );
    }
}
