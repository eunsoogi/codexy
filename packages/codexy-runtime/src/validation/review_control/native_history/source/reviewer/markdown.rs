use serde_json::{Map, Value, json};

use super::super::fields;

#[path = "markdown/context.rs"]
mod context;
#[path = "markdown/findings.rs"]
mod findings;
#[path = "markdown/result.rs"]
mod result;
#[path = "markdown/settings.rs"]
mod settings;

pub(super) fn line_ranges(raw: &str) -> Vec<(usize, usize, &str)> {
    context::line_ranges(raw)
}

pub(super) fn operative_lines(raw: &str) -> Vec<(usize, usize, &str)> {
    context::operative_lines(raw)
}

pub(super) fn operative_line_indices(lines: &[(usize, usize, &str)]) -> Vec<usize> {
    context::operative_line_indices(lines)
}

pub(super) fn heading(line: &str) -> Option<(usize, &str)> {
    context::heading(line)
}

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
        let leading = line.len() - line.trim_start().len();
        let mut content = &line[leading..];
        let mut content_start = leading;
        if let Some(rest) = content.strip_prefix("- ") {
            content = rest;
            content_start += 2;
        }
        let lower = content.to_ascii_lowercase();
        for marker in [
            "exact reviewed pr head",
            "exact current pr head",
            "exact current head",
            "at exact pr head",
        ] {
            if lower.starts_with(marker) {
                let (head, offset) = head_value(&content[marker.len()..])?;
                let start = start + content_start + marker.len() + offset;
                matches.push(Located {
                    value: head,
                    start,
                    end: start + 40,
                });
            }
        }
        if let Some((head, offset)) = labelled_head(line)? {
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
    let mut in_terminal_handoff = false;
    for (index, (start, _, line)) in lines.iter().enumerate() {
        let lower = line.trim().to_ascii_lowercase();
        if let Some((level, title)) = heading(line) {
            in_terminal_handoff = level == 2 && title.eq_ignore_ascii_case("terminal handoff");
        }
        let has_terminal_label = terminal_label(&lower, in_terminal_handoff);
        let value = if lower.starts_with("##") && lower.contains("blocking findings") {
            result_value(line).map(|(result, offset)| (result, *start + offset))
        } else if has_terminal_label {
            match line.split_once(':') {
                Some((prefix, value)) => result_value(value)
                    .map(|(result, offset)| (result, *start + prefix.len() + 1 + offset)),
                None => lines[index + 1..]
                    .iter()
                    .find(|(_, _, next)| !next.trim().is_empty())
                    .and_then(|(next_start, _, next)| {
                        result_value(next).map(|(result, offset)| (result, *next_start + offset))
                    }),
            }
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

fn terminal_label(line: &str, in_terminal_handoff: bool) -> bool {
    let line = line.trim();
    let line = line.strip_prefix('-').map_or(line, str::trim_start);
    let line = line.trim_start_matches('#').trim_start();
    let label = line.split_once(':').map_or(line, |(label, _)| label);
    matches!(label.trim(), "terminal result" | "terminal verdict")
        || (in_terminal_handoff && label.trim() == "result")
}

pub(super) fn labelled_head(line: &str) -> Result<Option<(String, usize)>, String> {
    let trimmed = line.trim().trim_start_matches("- ").trim();
    let Some((label, value)) = trimmed.split_once(':') else {
        return Ok(None);
    };
    let label = label.to_ascii_lowercase().replace(['-', '_'], " ");
    if !matches!(
        label.trim(),
        "exact head"
            | "exact reviewed pr head"
            | "exact current pr head"
            | "exact current head"
            | "reviewed head"
    ) {
        return Ok(None);
    }
    let (head, offset) = exact_sha_value(value)
        .ok_or_else(|| "markdown reviewed head label lacks a SHA".to_owned())?;
    let line_offset = line.find(value).unwrap_or_default();
    Ok(Some((head, line_offset + offset)))
}

fn head_value(value: &str) -> Result<(String, usize), String> {
    let leading = value.len() - value.trim_start().len();
    let mut start = leading;
    if value[start..].starts_with(':') {
        start += 1;
        start += value[start..].len() - value[start..].trim_start().len();
    }
    let (head, offset) = exact_sha_value(&value[start..])
        .ok_or_else(|| "markdown reviewed head label lacks a SHA".to_owned())?;
    Ok((head, start + offset))
}

pub(super) fn exact_sha_value(value: &str) -> Option<(String, usize)> {
    let leading = value.len() - value.trim_start().len();
    let value = &value[leading..];
    let head = value.trim_matches(|character: char| "`.,;:)]".contains(character));
    fields::is_sha(head).then(|| {
        (
            head.to_owned(),
            leading + value.find(head).unwrap_or_default(),
        )
    })
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

pub(super) fn result_value(value: &str) -> Option<(String, usize)> {
    result::parse(value)
}
