use std::path::Path;

pub(super) fn visible_lines(path: &Path, text: &str) -> Vec<Option<String>> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => rust_lines(text),
        Some("sh") => awk_lines(text),
        Some("md") => markdown_lines(text),
        _ => text.lines().map(|line| Some(line.to_owned())).collect(),
    }
}

fn rust_lines(text: &str) -> Vec<Option<String>> {
    let mut state = RustState::Code;
    text.lines()
        .map(|line| strip_rust_spans(line, &mut state))
        .collect()
}

enum RustState {
    Code,
    BlockComment,
    Raw(String),
}

fn strip_rust_spans(line: &str, state: &mut RustState) -> Option<String> {
    let mut visible = String::new();
    let mut remainder = line;
    loop {
        match state {
            RustState::Raw(end) => {
                let index = remainder.find(end.as_str())?;
                remainder = &remainder[index + end.len()..];
                *state = RustState::Code;
            }
            RustState::BlockComment => {
                let index = remainder.find("*/")?;
                remainder = &remainder[index + 2..];
                *state = RustState::Code;
            }
            RustState::Code => {
                let next = next_rust_span(remainder);
                let Some((index, span)) = next else {
                    visible.push_str(remainder);
                    return Some(visible);
                };
                visible.push_str(&remainder[..index]);
                match span {
                    RustSpan::LineComment => return Some(visible),
                    RustSpan::BlockComment => {
                        remainder = &remainder[index + 2..];
                        *state = RustState::BlockComment;
                    }
                    RustSpan::Raw(end) => {
                        remainder = &remainder[index + end.len()..];
                        if let Some(closing) = remainder.find(&end) {
                            remainder = &remainder[closing + end.len()..];
                        } else {
                            *state = RustState::Raw(end);
                            return Some(visible);
                        }
                    }
                }
            }
        }
    }
}

enum RustSpan {
    LineComment,
    BlockComment,
    Raw(String),
}

fn next_rust_span(line: &str) -> Option<(usize, RustSpan)> {
    let bytes = line.as_bytes();
    for index in 0..bytes.len() {
        if bytes[index..].starts_with(b"//") {
            return Some((index, RustSpan::LineComment));
        }
        if bytes[index..].starts_with(b"/*") {
            return Some((index, RustSpan::BlockComment));
        }
        if bytes[index] != b'r' {
            continue;
        }
        let hashes = bytes[index + 1..]
            .iter()
            .take_while(|byte| **byte == b'#')
            .count();
        if bytes.get(index + hashes + 1) == Some(&b'"') {
            return Some((index, RustSpan::Raw(format!("\"{}", "#".repeat(hashes)))));
        }
    }
    None
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
    let Some(index) = awk_program_opener(line) else {
        return Some(line.to_owned());
    };
    let prefix = &line[..index];
    let after_quote = &line[index + 1..];
    if let Some(closing) = after_quote.find('\'') {
        let mut visible = prefix.to_owned();
        visible.push_str(&after_quote[closing + 1..]);
        return Some(visible);
    }
    *quoted = true;
    Some(prefix.to_owned())
}

fn awk_program_opener(line: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(index) = line[offset..].find("awk") {
        let index = offset + index;
        let before = line[..index].chars().next_back();
        let after = line[index + 3..].chars().next();
        if before.is_none_or(|character| !is_shell_word(character))
            && after.is_none_or(|character| !is_shell_word(character))
        {
            if let Some(quote) = line[index + 3..].find('\'') {
                return Some(index + 3 + quote);
            }
        }
        offset = index + 3;
    }
    None
}

fn is_shell_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn markdown_lines(text: &str) -> Vec<Option<String>> {
    let mut fence = None;
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let marker = trimmed
                .chars()
                .next()
                .filter(|marker| matches!(marker, '`' | '~'));
            if let Some(marker) =
                marker.filter(|marker| trimmed.starts_with(&marker.to_string().repeat(3)))
            {
                fence = if fence == Some(marker) {
                    None
                } else {
                    Some(marker)
                };
                None
            } else if fence.is_some()
                || trimmed.starts_with('|')
                || trimmed.matches('|').count() >= 2
            {
                None
            } else {
                Some(line.to_owned())
            }
        })
        .collect()
}
