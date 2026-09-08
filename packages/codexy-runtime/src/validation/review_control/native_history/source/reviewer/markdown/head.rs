use super::super::fields;
use super::{Located, context};

pub(super) fn reviewed_head(raw: &str) -> Result<Option<Located>, String> {
    let mut matches = Vec::new();
    let lines = context::operative_lines(raw);
    let terminal_control = context::section_membership(
        &lines,
        |_| true,
        |level, title| level == 2 && title.eq_ignore_ascii_case("terminal control"),
    );
    for (index, (start, _, line)) in lines.iter().enumerate() {
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
                let start = *start + content_start + marker.len() + offset;
                matches.push(Located {
                    value: head,
                    start,
                    end: start + 40,
                });
            }
        }
        if let Some((head, offset)) = labelled_head(line)? {
            let start = *start + offset;
            matches.push(Located {
                value: head,
                start,
                end: start + 40,
            });
        }
        if let Some((_, head)) = terminal_history_entry(line, terminal_control[index])? {
            matches.push(Located {
                value: head.value,
                start: *start + head.start,
                end: *start + head.end,
            });
        }
    }
    unique(matches, "markdown reviewed head")
}

pub(super) fn terminal_history_kind(raw: &str) -> Result<Option<Located>, String> {
    let lines = context::operative_lines(raw);
    let terminal_control = context::section_membership(
        &lines,
        |_| true,
        |level, title| level == 2 && title.eq_ignore_ascii_case("terminal control"),
    );
    let mut matches = Vec::new();
    for (index, (line_start, _, line)) in lines.iter().enumerate() {
        if let Some((mut kind, _)) = terminal_history_entry(line, terminal_control[index])? {
            kind.start += *line_start;
            kind.end += *line_start;
            matches.push(kind);
        }
    }
    unique(matches, "markdown review kind")
}

fn terminal_history_entry(
    line: &str,
    in_terminal_control: bool,
) -> Result<Option<(Located, Located)>, String> {
    if !in_terminal_control {
        return Ok(None);
    }
    let Some((content, content_start)) = numbered_content(line) else {
        return Ok(None);
    };
    let lower = content.to_ascii_lowercase();
    let marker = "required-current-head";
    if !lower.starts_with(marker) {
        return Ok(None);
    }
    let kind = Located {
        value: "required_current_head".into(),
        start: content_start,
        end: content_start + marker.len(),
    };
    let at = lower
        .find(" at ")
        .ok_or("markdown required-current-head entry lacks a reviewed head")?;
    let (head, offset) = exact_sha_value(&content[at + 4..])
        .ok_or("markdown required-current-head entry lacks a reviewed head")?;
    let start = content_start + at + 4 + offset;
    Ok(Some((
        kind,
        Located {
            value: head,
            start,
            end: start + 40,
        },
    )))
}

fn numbered_content(line: &str) -> Option<(&str, usize)> {
    let leading = line.len() - line.trim_start().len();
    let mut content = &line[leading..];
    let mut start = leading;
    if let Some(rest) = content.strip_prefix("- ") {
        content = rest;
        start += 2;
    }
    let dot = content.find('.')?;
    if dot == 0 || !content[..dot].bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let rest = &content[dot + 1..];
    let whitespace = rest.len() - rest.trim_start().len();
    Some((&rest[whitespace..], start + dot + 1 + whitespace))
}

pub(super) fn labelled_head(line: &str) -> Result<Option<(String, usize)>, String> {
    let trimmed = line.trim().trim_start_matches("- ").trim();
    let Some((label, value)) = trimmed.split_once(':') else {
        return Ok(None);
    };
    let label = label
        .trim()
        .trim_matches('`')
        .trim()
        .to_ascii_lowercase()
        .replace(['-', '_'], " ");
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
