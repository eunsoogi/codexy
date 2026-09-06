use serde_json::{Map, Value, json};

use super::super::fields;

#[path = "markdown/findings.rs"]
mod findings;
#[path = "markdown/settings.rs"]
mod settings;

pub(super) fn candidate(raw: &str) -> Result<Option<Map<String, Value>>, String> {
    let mut result = Map::new();
    let mut spans = Map::new();
    if let Some(head) = reviewed_head(raw)? {
        let span = head.span();
        result.insert("reviewed_head".into(), Value::String(head.value));
        spans.insert("reviewed_head".into(), span);
    }
    if let Some(terminal) = terminal_result(raw)? {
        let span = terminal.span();
        result.insert("terminal_result".into(), Value::String(terminal.value));
        spans.insert("terminal_result".into(), span);
    }
    if let Some(setting) = settings::reviewer_setting(raw)? {
        let span = setting.span();
        result.insert("model".into(), Value::String(setting.model));
        result.insert("reasoningEffort".into(), Value::String(setting.effort));
        spans.insert("reviewer".into(), span);
    }
    let findings = findings::values(raw)?;
    if !findings.is_empty() {
        result.insert("findings".into(), Value::Array(findings));
        spans.insert(
            "findings".into(),
            json!({"source": "final_message_markdown"}),
        );
    }
    if result.is_empty() {
        return Ok(None);
    }
    result.insert("sourceSpans".into(), Value::Object(spans));
    Ok(Some(result))
}

struct Located {
    value: String,
    start: usize,
    end: usize,
}

impl Located {
    fn span(&self) -> Value {
        json!({
            "source": "final_message_markdown",
            "start": self.start,
            "end": self.end,
            "coordinate": "utf8_bytes"
        })
    }
}

struct Setting {
    model: String,
    effort: String,
    start: usize,
    end: usize,
}

impl Setting {
    fn span(&self) -> Value {
        json!({
            "source": "final_message_markdown",
            "start": self.start,
            "end": self.end,
            "coordinate": "utf8_bytes"
        })
    }
}

fn reviewed_head(raw: &str) -> Result<Option<Located>, String> {
    let mut matches = Vec::new();
    for (start, _, line) in operative_lines(raw) {
        let lower = line.to_ascii_lowercase();
        for marker in ["exact reviewed pr head", "at exact pr head"] {
            if let Some(index) = lower.find(marker) {
                let Some((head, offset)) = find_sha(&line[index + marker.len()..]) else {
                    return Err("markdown reviewed head label lacks a SHA".into());
                };
                let start = start + index + marker.len() + offset;
                matches.push(Located {
                    value: head,
                    start,
                    end: start + 40,
                });
            }
        }
        if let Some((head, offset)) = labelled_sha(line) {
            let start = start + offset;
            matches.push(Located {
                value: head,
                start,
                end: start + 40,
            });
        }
    }
    unique(matches, "markdown reviewed head")
}

fn terminal_result(raw: &str) -> Result<Option<Located>, String> {
    let mut matches = Vec::new();
    let lines = operative_lines(raw);
    for (index, (start, _, line)) in lines.iter().enumerate() {
        let lower = line.trim().to_ascii_lowercase();
        let has_terminal_label = terminal_label(&lower);
        let value = if lower.starts_with("##") && lower.contains("blocking findings") {
            find_result(line).map(|(result, offset)| (result, *start + offset))
        } else if has_terminal_label {
            line.split_once(':')
                .and_then(|(prefix, value)| {
                    find_result(value)
                        .map(|(result, offset)| (result, *start + prefix.len() + 1 + offset))
                })
                .or_else(|| {
                    lines[index + 1..]
                        .iter()
                        .find(|(_, _, next)| !next.trim().is_empty())
                        .and_then(|(next_start, _, next)| {
                            find_result(next).map(|(result, offset)| (result, *next_start + offset))
                        })
                })
        } else {
            None
        };
        if has_terminal_label && value.is_none() {
            return Err("markdown terminal result label is invalid".into());
        }
        if let Some((value, offset)) = value {
            let length = value.len();
            matches.push(Located {
                value,
                start: offset,
                end: offset + length,
            });
        }
    }
    unique(matches, "markdown terminal result")
}

fn terminal_label(line: &str) -> bool {
    ["terminal result", "terminal verdict"]
        .iter()
        .any(|label| line.contains(label))
}

fn labelled_sha(line: &str) -> Option<(String, usize)> {
    let trimmed = line.trim().trim_start_matches("- ").trim();
    let (label, value) = trimmed.split_once(':')?;
    let label = label.to_ascii_lowercase().replace(['-', '_'], " ");
    if !matches!(
        label.trim(),
        "exact head" | "exact reviewed pr head" | "reviewed head"
    ) {
        return None;
    }
    let (head, offset) = find_sha(value)?;
    let line_offset = line.find(value)?;
    Some((head, line_offset + offset))
}

fn unique(matches: Vec<Located>, label: &str) -> Result<Option<Located>, String> {
    let mut matches = matches.into_iter();
    let Some(first) = matches.next() else {
        return Ok(None);
    };
    if matches.any(|candidate| candidate.value != first.value) {
        return Err(format!("{label} is contradictory"));
    }
    Ok(Some(first))
}

pub(super) fn operative_lines(raw: &str) -> Vec<(usize, usize, &str)> {
    let mut in_fence = false;
    let mut start = 0;
    let mut result = Vec::new();
    for part in raw.split_inclusive('\n') {
        let without_newline = part.strip_suffix('\n').unwrap_or(part);
        let line = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        let trimmed = line.trim_start();
        let end = start + line.len();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && !trimmed.starts_with('>') {
            result.push((start, end, line));
        }
        start += part.len();
    }
    result
}

fn find_result(value: &str) -> Option<(String, usize)> {
    let mut start = None;
    for (index, character) in value
        .char_indices()
        .chain(std::iter::once((value.len(), '\0')))
    {
        if character.is_ascii_alphabetic() {
            start.get_or_insert(index);
        } else if let Some(start) = start.take() {
            let candidate = value[start..index].to_ascii_uppercase();
            if ["PASS", "BLOCK", "UNOBSERVABLE"].contains(&candidate.as_str()) {
                return Some((candidate, start));
            }
        }
    }
    None
}

fn find_sha(value: &str) -> Option<(String, usize)> {
    let mut start = None;
    for (index, character) in value
        .char_indices()
        .chain(std::iter::once((value.len(), '\0')))
    {
        if character.is_ascii_hexdigit() {
            start.get_or_insert(index);
        } else if let Some(start) = start.take() {
            let part = &value[start..index];
            if fields::is_sha(part) {
                return Some((part.to_owned(), start));
            }
        }
    }
    None
}

fn clean(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character: char| character == '`' || ",.;()[]".contains(character))
        .to_owned()
}
