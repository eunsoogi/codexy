use serde_json::{Value, json};

#[path = "followups.rs"]
mod followups;

pub(super) fn values(raw: &str) -> Result<Vec<Value>, String> {
    let lines = line_ranges(raw);
    let starts = operative_headers(&lines);
    let mut result = Vec::new();
    for (position, line_index) in starts.iter().enumerate() {
        let (start, _, line) = lines[*line_index];
        let mut end = starts
            .get(position + 1)
            .map(|next| lines[*next].0)
            .unwrap_or(raw.len());
        for (line_start, _, next_line) in &lines[*line_index + 1..] {
            if *line_start < end && next_line.trim_start().starts_with("## ") {
                end = *line_start;
                break;
            }
        }
        let block = raw[start..end].trim_end();
        let (severity, disposition) =
            finding_header(line).ok_or("markdown finding header changed")?;
        let paths = explicit_paths(block);
        let mut value = json!({
            "text": block,
            "severity": severity.to_ascii_lowercase(),
            "disposition": disposition,
            "sourceSpan": {
                "source": "final_message_markdown",
                "start": start,
                "end": start + block.len(),
                "coordinate": "utf8_bytes"
            }
        });
        let Some(map) = value.as_object_mut() else {
            return Err("markdown finding is not an object".into());
        };
        if let Some(path) = paths.first() {
            map.insert("path".into(), Value::String(path.clone()));
            map.insert("paths".into(), json!(paths));
        }
        if let Some(kind) = explicit_label(block, &["semantic kind", "semantic_kind"])? {
            map.insert("semanticKind".into(), Value::String(kind));
        }
        if let Some(source) = explicit_label(block, &["finding source", "source"])? {
            map.insert("source".into(), Value::String(source));
        }
        result.push(value);
    }
    result.extend(followup_values(raw, &lines)?);
    result.sort_by_key(|value| value["sourceSpan"]["start"].as_u64());
    Ok(result)
}

fn line_ranges(raw: &str) -> Vec<(usize, usize, &str)> {
    let mut start = 0;
    let mut result = Vec::new();
    for part in raw.split_inclusive('\n') {
        let without_newline = part.strip_suffix('\n').unwrap_or(part);
        let line = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        let end = start + line.len();
        result.push((start, end, line));
        start += part.len();
    }
    if raw.is_empty() {
        result.push((0, 0, raw));
    }
    result
}

fn operative_headers(lines: &[(usize, usize, &str)]) -> Vec<usize> {
    let mut in_fence = false;
    let mut result = Vec::new();
    for (index, (_, _, line)) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && !trimmed.starts_with('>') && finding_header(line).is_some() {
            result.push(index);
        }
    }
    result
}

fn finding_header(line: &str) -> Option<(String, String)> {
    let line = line.trim_start();
    let dot = line.find('.')?;
    if dot == 0 || !line[..dot].bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let rest = line[dot + 1..].trim_start();
    let body = rest.strip_prefix("**")?;
    let close = body.find("**")?;
    let mut heading = body[..close].trim();
    let tail = body[close + 2..].trim_start();
    let disposition = if let Some(disposition) = leading_parenthetical(tail) {
        disposition
    } else {
        let (open, disposition) = trailing_parenthetical(heading)?;
        heading = heading[..open].trim_end();
        disposition
    };
    let severity = heading
        .split_once('—')
        .or_else(|| heading.split_once(" - "))?
        .0
        .trim();
    (!severity.is_empty() && !disposition.is_empty())
        .then(|| (severity.to_owned(), disposition.to_owned()))
}

fn leading_parenthetical(value: &str) -> Option<&str> {
    let value = value.strip_prefix('(')?;
    let close = value.find(')')?;
    let disposition = value[..close].trim().trim_matches(char::from(96)).trim();
    (!disposition.is_empty()).then_some(disposition)
}

fn trailing_parenthetical(value: &str) -> Option<(usize, &str)> {
    let value = value.trim_end_matches(|character: char| {
        character.is_whitespace() || matches!(character, '.' | ';' | ':')
    });
    let open = value.rfind('(')?;
    let close = value[open + 1..].find(')')? + open + 1;
    if !value[close + 1..].trim().is_empty() {
        return None;
    }
    let disposition = value[open + 1..close]
        .trim()
        .trim_matches(char::from(96))
        .trim();
    (!disposition.is_empty()).then_some((open, disposition))
}

fn followup_header(line: &str) -> Option<String> {
    let line = line.trim_start().strip_prefix("- ")?;
    line.split(char::from(96))
        .find(|value| {
            !value.is_empty()
                && value
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
                && value.contains('_')
        })
        .map(str::to_owned)
}

fn followup_values(raw: &str, lines: &[(usize, usize, &str)]) -> Result<Vec<Value>, String> {
    followups::values(raw, lines)
}

pub(super) fn explicit_paths(block: &str) -> Vec<String> {
    let mut result = Vec::new();
    for line in operative_text_lines(block) {
        let mut cursor = 0;
        while let Some(open_offset) = line[cursor..].find('[') {
            let open = cursor + open_offset;
            let Some(close_offset) = line[open + 1..].find(']') else {
                break;
            };
            let close = open + 1 + close_offset;
            let Some(url_start) = line[close + 1..].strip_prefix('(') else {
                cursor = close + 1;
                continue;
            };
            let Some(url_end) = url_start.find(')') else {
                break;
            };
            if let Some(path) = clean_path(&line[open + 1..close]) {
                if !result.contains(&path) {
                    result.push(path);
                }
            }
            cursor = close + 2 + url_end;
        }
    }
    result
}

fn operative_text_lines(raw: &str) -> Vec<&str> {
    let mut in_fence = false;
    let mut result = Vec::new();
    for part in raw.split_inclusive('\n') {
        let without_newline = part.strip_suffix('\n').unwrap_or(part);
        let line = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && !trimmed.starts_with('>') {
            result.push(line);
        }
    }
    result
}

pub(super) fn clean_path(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('`');
    let value = value
        .rsplit_once(':')
        .filter(|(_, suffix)| suffix.bytes().all(|byte| byte.is_ascii_digit()))
        .map_or(value, |(path, _)| path);
    let path = value.trim();
    let extension = path.rsplit_once('.').map(|(_, extension)| extension);
    if path.is_empty()
        || path.contains(char::is_whitespace)
        || (!path.contains('/')
            && !matches!(extension, Some("rs" | "py" | "cmd" | "sh" | "toml" | "md")))
    {
        return None;
    }
    Some(path.to_owned())
}

fn explicit_label(block: &str, labels: &[&str]) -> Result<Option<String>, String> {
    let mut result = None;
    for line in operative_text_lines(block) {
        let Some((label, value)) = line.split_once(':') else {
            continue;
        };
        let label = label
            .trim()
            .trim_start_matches('-')
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], " ");
        if labels.contains(&label.trim()) {
            let value = clean(value);
            if result.as_ref().is_some_and(|prior| prior != &value) {
                return Err(format!("markdown {} is contradictory", labels[0]));
            }
            result = Some(value);
        }
    }
    Ok(result)
}

fn clean(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character: char| character == '`' || ",.;()[]".contains(character))
        .to_owned()
}
