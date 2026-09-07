use serde_json::{Value, json};

#[path = "followups.rs"]
mod followups;
#[path = "paths.rs"]
mod paths;

pub(super) fn values(raw: &str) -> Result<Vec<Value>, String> {
    let lines = super::line_ranges(raw);
    let mut operative = vec![false; lines.len()];
    for index in super::operative_line_indices(&lines) {
        operative[index] = true;
    }
    let starts = operative_headers(&lines);
    let mut result = Vec::new();
    for (position, line_index) in starts.iter().enumerate() {
        let (start, _, line) = lines[*line_index];
        let mut end = starts
            .get(position + 1)
            .map(|next| lines[*next].0)
            .unwrap_or(raw.len());
        for (offset, (line_start, _, next_line)) in lines[*line_index + 1..].iter().enumerate() {
            let next_index = *line_index + 1 + offset;
            if *line_start < end && operative[next_index] && super::heading(next_line).is_some() {
                end = *line_start;
                break;
            }
        }
        let block = raw[start..end].trim_end();
        let (severity, disposition) =
            finding_header(line).ok_or("markdown finding header changed")?;
        let paths = paths::explicit_paths(block);
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

fn operative_headers(lines: &[(usize, usize, &str)]) -> Vec<usize> {
    let mut result = Vec::new();
    for index in super::operative_line_indices(lines) {
        let line = lines[index].2;
        if finding_header(line).is_some() {
            result.push(index);
        }
    }
    result
}

fn finding_header(line: &str) -> Option<(String, String)> {
    let line = line.trim_start();
    let line = strip_number(line);
    let body = line.strip_prefix("**")?;
    let close = body.find("**")?;
    let mut heading = body[..close].trim();
    let tail = body[close + 2..].trim_start();
    let disposition = if let Some(disposition) = leading_parenthetical(tail) {
        disposition
    } else if let Some(disposition) = leading_em_dash_label(tail) {
        disposition
    } else {
        if let Some((open, disposition)) = trailing_parenthetical(heading) {
            heading = heading[..open].trim_end();
            disposition
        } else {
            let (open, disposition) = trailing_em_dash_label(heading)?;
            heading = heading[..open].trim_end();
            disposition
        }
    };
    let severity = heading
        .split_once('—')
        .or_else(|| heading.split_once(" - "))?
        .0
        .trim();
    (!severity.is_empty() && !disposition.is_empty())
        .then(|| (severity.to_owned(), disposition.to_owned()))
}

fn strip_number(line: &str) -> &str {
    let line = line.strip_prefix("- ").unwrap_or(line);
    let Some(dot) = line.find('.') else {
        return line;
    };
    if dot == 0 || !line[..dot].bytes().all(|byte| byte.is_ascii_digit()) {
        return line;
    }
    line[dot + 1..].trim_start()
}

fn leading_parenthetical(value: &str) -> Option<&str> {
    let value = value.strip_prefix('(')?;
    let close = value.find(')')?;
    let disposition = value[..close].trim().trim_matches(char::from(96)).trim();
    (!disposition.is_empty()).then_some(disposition)
}

fn leading_em_dash_label(value: &str) -> Option<&str> {
    let value = value.strip_prefix('—')?.trim_start();
    let value = value.strip_prefix('`')?;
    let close = value.find('`')?;
    if !value[close + 1..].trim().is_empty() {
        return None;
    }
    let disposition = value[..close].trim();
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

fn trailing_em_dash_label(value: &str) -> Option<(usize, &str)> {
    let open = value.rfind(" — ")?;
    let disposition = value[open + " — ".len()..]
        .trim()
        .trim_matches(char::from(96))
        .trim();
    if disposition.is_empty()
        || !disposition
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return None;
    }
    Some((open, disposition))
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

fn explicit_label(block: &str, labels: &[&str]) -> Result<Option<String>, String> {
    let mut result = None;
    for line in paths::operative_text_lines(block) {
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
