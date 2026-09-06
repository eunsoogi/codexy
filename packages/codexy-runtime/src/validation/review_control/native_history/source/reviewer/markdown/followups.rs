use serde_json::{Value, json};

use super::{followup_header, paths};
use paths::{clean_path, explicit_paths};

pub(super) fn values(raw: &str, lines: &[(usize, usize, &str)]) -> Result<Vec<Value>, String> {
    let mut in_fence = false;
    let mut in_followup = false;
    let mut result = Vec::new();
    for (index, (start, _, line)) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if super::super::fence_marker(trimmed) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.starts_with('>') {
            continue;
        }
        if let Some((level, title)) = super::super::heading(line) {
            in_followup = level == 2
                && title
                    .to_ascii_lowercase()
                    .contains("non-blocking follow-up");
            continue;
        }
        if !in_followup {
            continue;
        }
        let Some(disposition) = followup_header(line) else {
            continue;
        };
        let end = lines[index + 1..]
            .iter()
            .find(|(_, _, next)| {
                let next = next.trim_start();
                next.starts_with("- ") || super::super::heading(next).is_some()
            })
            .map_or(raw.len(), |(start, _, _)| *start);
        let block = raw[*start..end].trim_end();
        let mut paths = explicit_paths(block);
        if paths.is_empty() {
            paths = block.split(char::from(96)).filter_map(clean_path).collect();
        }
        let mut value = json!({
            "text": block,
            "disposition": disposition,
            "sourceSpan": {
                "source": "final_message_markdown",
                "start": start,
                "end": start + block.len(),
                "coordinate": "utf8_bytes"
            }
        });
        let Some(map) = value.as_object_mut() else {
            return Err("markdown follow-up is not an object".into());
        };
        if let Some(path) = paths.first() {
            map.insert("path".into(), Value::String(path.clone()));
            map.insert("paths".into(), json!(paths));
        }
        result.push(value);
    }
    Ok(result)
}
