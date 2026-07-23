use std::path::Path;

const INACTIVE_HTML_TAGS: &[&str] = &["pre", "code", "script", "style", "textarea", "template"];

pub(super) fn contract_text(path: &Path, text: &str) -> Result<String, &'static str> {
    if path.extension().and_then(|extension| extension.to_str()) == Some("toml") {
        return toml_contract_text(text);
    }
    Ok(normative_markdown(text))
}

fn toml_contract_text(text: &str) -> Result<String, &'static str> {
    let document = toml::from_str::<toml::Value>(text)
        .map_err(|_| "TOML instruction source must parse before contract validation")?;
    let prompt = document
        .get("developer_instructions")
        .and_then(toml::Value::as_str)
        .ok_or("TOML instruction source must contain developer_instructions")?;
    Ok(normative_markdown(prompt))
}

pub(super) fn normative_markdown(text: &str) -> String {
    let mut output = String::new();
    let mut fence = None;
    let mut in_comment = false;
    let mut html_code = None;
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if let Some((marker, length)) = fence {
            if closes_fence(trimmed, marker, length) {
                fence = None;
            }
            continue;
        }
        if !in_comment && html_code.is_none() && is_indented_code(raw_line) {
            continue;
        }

        let visible = if html_code.is_some() {
            without_html_code_blocks(raw_line, &mut html_code)
        } else {
            let unquoted = without_markdown_literals(raw_line);
            let uncommented = without_html_comments(&unquoted, &mut in_comment);
            without_html_code_blocks(&uncommented, &mut html_code)
        };
        let visible = without_html_comments(&visible, &mut in_comment);
        let visible_trimmed = visible.trim();
        if let Some(opening) = opening_fence(visible_trimmed) {
            fence = Some(opening);
            continue;
        }
        if !visible_trimmed.is_empty() {
            output.push_str(&visible);
            output.push('\n');
        }
    }
    output
}

pub(super) fn contains_clause(text: &str, clause: &str) -> bool {
    let clause = normalize(clause);
    text.lines().any(|line| {
        let line = normalize(line);
        line.match_indices(&clause).any(|(index, matched)| {
            is_statement_prefix(&line[..index]) && line[index + matched.len()..].trim().is_empty()
        })
    })
}

fn is_indented_code(line: &str) -> bool {
    line.starts_with("    ") || line.starts_with('\t')
}

fn is_statement_prefix(prefix: &str) -> bool {
    let prefix = prefix
        .rsplit_once(". ")
        .map_or(prefix, |(_, statement)| statement)
        .trim();
    matches!(prefix, "" | "-" | "+" | "*")
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn without_markdown_literals(line: &str) -> String {
    without_inline_code(&without_backslash_escapes(line))
}

fn without_backslash_escapes(line: &str) -> String {
    let mut output = String::new();
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\\' && characters.peek().is_some_and(char::is_ascii_punctuation) {
            output.push(' ');
            output.push(' ');
            characters.next();
        } else {
            output.push(character);
        }
    }
    output
}

fn without_inline_code(line: &str) -> String {
    let mut output = String::new();
    let mut remainder = line;
    while let Some(open) = remainder.find('`') {
        let width = remainder[open..]
            .bytes()
            .take_while(|byte| *byte == b'`')
            .count();
        let content = &remainder[open + width..];
        let Some(close) = matching_backtick_run(content, width) else {
            output.push_str(remainder);
            return output;
        };
        output.push_str(&remainder[..open + width]);
        for character in content[..close].chars() {
            output.push(if matches!(character, '<' | '>') {
                ' '
            } else {
                character
            });
        }
        output.push_str(&content[close..close + width]);
        remainder = &content[close + width..];
    }
    output.push_str(remainder);
    output
}

fn matching_backtick_run(text: &str, expected: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index] == b'`' {
            index += 1;
        }
        if index - start == expected {
            return Some(start);
        }
    }
    None
}

fn without_html_code_blocks(line: &str, state: &mut Option<&'static str>) -> String {
    let mut output = String::new();
    let mut remainder = line;
    loop {
        if let Some(tag) = state {
            let closing = format!("</{tag}");
            let lower = remainder.to_ascii_lowercase();
            let Some(start) = lower.match_indices(&closing).find_map(|(index, matched)| {
                is_html_tag_boundary(&lower[index + matched.len()..]).then_some(index)
            }) else {
                break;
            };
            let Some(end) = remainder[start..].find('>') else {
                break;
            };
            remainder = &remainder[start + end + 1..];
            *state = None;
            continue;
        }
        let Some((start, tag)) = opening_html_code_tag(remainder) else {
            output.push_str(remainder);
            break;
        };
        output.push_str(&remainder[..start]);
        *state = Some(tag);
        let Some(end) = remainder[start..].find('>') else {
            break;
        };
        remainder = &remainder[start + end + 1..];
    }
    output
}

fn opening_html_code_tag(line: &str) -> Option<(usize, &'static str)> {
    let lower = line.to_ascii_lowercase();
    INACTIVE_HTML_TAGS
        .iter()
        .copied()
        .filter_map(|tag| {
            lower
                .match_indices(&format!("<{tag}"))
                .find_map(|(index, matched)| {
                    is_html_tag_boundary(&lower[index + matched.len()..]).then_some((index, tag))
                })
        })
        .min_by_key(|(index, _)| *index)
}

fn is_html_tag_boundary(remainder: &str) -> bool {
    remainder
        .chars()
        .next()
        .is_none_or(|next| next.is_ascii_whitespace() || next == '>')
}

fn without_html_comments(line: &str, in_comment: &mut bool) -> String {
    let mut output = String::new();
    let mut remainder = line;
    loop {
        if *in_comment {
            let Some(end) = remainder.find("-->") else {
                break;
            };
            remainder = &remainder[end + 3..];
            *in_comment = false;
            continue;
        }
        let Some(start) = remainder.find("<!--") else {
            output.push_str(remainder);
            break;
        };
        output.push_str(&remainder[..start]);
        remainder = &remainder[start + 4..];
        *in_comment = true;
    }
    output
}

fn opening_fence(line: &str) -> Option<(char, usize)> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = line
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (length >= 3).then_some((marker, length))
}

fn closes_fence(line: &str, marker: char, opening_length: usize) -> bool {
    let length = line
        .chars()
        .take_while(|character| *character == marker)
        .count();
    length >= opening_length && line[length..].trim().is_empty()
}
