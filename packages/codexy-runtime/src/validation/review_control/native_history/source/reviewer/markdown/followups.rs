use serde_json::{Value, json};

use super::{followup_header, paths};
use paths::{clean_path, explicit_paths};

pub(super) fn values(raw: &str, lines: &[(usize, usize, &str)]) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    let mut operative_lines = vec![false; lines.len()];
    for index in super::super::operative_line_indices(lines) {
        operative_lines[index] = true;
    }
    let followup_section = super::super::context::section_membership(
        lines,
        |index| operative_lines[index],
        |level, title| {
            level == 2
                && title
                    .to_ascii_lowercase()
                    .contains("non-blocking follow-up")
        },
    );
    for (index, (start, _, line)) in lines.iter().enumerate() {
        if !operative_lines[index] {
            continue;
        }
        if !followup_section[index] {
            continue;
        }
        let Some(disposition) = followup_header(line) else {
            continue;
        };
        let end = lines[index + 1..]
            .iter()
            .enumerate()
            .find(|(offset, (_, _, next))| {
                let next_index = index + 1 + offset;
                operative_lines[next_index] && {
                    let next = next.trim_start();
                    next.starts_with("- ") || super::super::heading(next).is_some()
                }
            })
            .map_or(raw.len(), |(_, (start, _, _))| *start);
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
