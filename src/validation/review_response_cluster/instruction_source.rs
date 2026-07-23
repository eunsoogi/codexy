use std::path::Path;

pub(super) fn contract_text(path: &Path, text: &str) -> Result<String, &'static str> {
    if path.extension().and_then(|extension| extension.to_str()) == Some("toml") {
        return toml_contract_text(text);
    }
    Ok(normative_markdown(text))
}

fn toml_contract_text(text: &str) -> Result<String, &'static str> {
    let document = toml::from_str::<toml::Value>(text)
        .map_err(|_| "TOML instruction source must parse before contract validation")?;
    document
        .get("developer_instructions")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or("TOML instruction source must contain developer_instructions")
}

fn normative_markdown(text: &str) -> String {
    let mut output = String::new();
    let mut fence = None;
    let mut in_comment = false;
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if let Some((marker, length)) = fence {
            if closes_fence(trimmed, marker, length) {
                fence = None;
            }
            continue;
        }

        let visible = without_html_comments(raw_line, &mut in_comment);
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
