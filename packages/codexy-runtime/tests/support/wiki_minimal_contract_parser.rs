use std::collections::BTreeMap;

use crate::support::wiki_minimal_contract_activity::ActiveMarkdown;

pub(crate) fn section(text: &str, title: &str) -> Result<String, String> {
    let (mut fence, mut active) = (None, ActiveMarkdown::default());
    let mut count = 0;
    for raw in text.lines() {
        let line = active.line(raw, fence.is_some())?;
        transition(&mut fence, &line);
        if fence.is_none() && line == title {
            count += 1;
        }
    }
    active.finish()?;
    balanced(fence)?;
    if count != 1 {
        return Err(format!("missing or duplicate section {title}"));
    }
    let level = title
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let mut body = None;
    (fence, active) = (None, ActiveMarkdown::default());
    for raw in text.lines() {
        let line = active.line(raw, fence.is_some())?;
        transition(&mut fence, &line);
        if fence.is_none() && line == title {
            body = Some(String::new());
            continue;
        }
        let next_level = line
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if body.is_some()
            && fence.is_none()
            && next_level > 0
            && next_level <= level
            && line.as_bytes().get(next_level) == Some(&b' ')
        {
            break;
        }
        if let Some(body) = &mut body {
            body.push_str(&line);
            body.push('\n');
        }
    }
    active.finish()?;
    balanced(fence)?;
    body.ok_or_else(|| format!("missing section {title}"))
}

pub(crate) fn workflow_rows(table: &str) -> Result<BTreeMap<String, String>, String> {
    let mut rows: Vec<String> = Vec::new();
    let (mut fence, mut active) = (None, ActiveMarkdown::default());
    let mut table_ended = false;
    for raw in table.lines() {
        let line = active.line(raw, fence.is_some())?;
        transition(&mut fence, &line);
        if let Some(row) = workflow_row(&line) {
            if fence.is_some() {
                return Err("workflow table rows cannot be fenced".into());
            }
            if table_ended {
                return Err("workflow table must be contiguous".into());
            }
            rows.push(row.into());
        } else if !rows.is_empty() {
            table_ended = true;
        }
    }
    active.finish()?;
    balanced(fence)?;
    if rows.len() < 3 || cells(&rows[0])? != ["Current workflow", "Disposition", "Contract role"] {
        return Err("invalid workflow table header".into());
    }
    if !separator(&rows[1])? {
        return Err("invalid workflow table separator".into());
    }
    let mut workflows = BTreeMap::new();
    for row in &rows[2..] {
        let cells = cells(row)?;
        if cells.len() != 3 || cells[2].is_empty() {
            return Err("malformed workflow row".into());
        }
        let name = cells[0]
            .strip_prefix('`')
            .and_then(|name| name.strip_suffix('`'))
            .ok_or("workflow name must be code-formatted")?;
        if !matches!(cells[1], "Keep" | "Merge" | "Remove")
            || workflows.insert(name.into(), cells[1].into()).is_some()
        {
            return Err("duplicate or invalid workflow disposition".into());
        }
    }
    Ok(workflows)
}

pub(crate) fn assignments(section: &str) -> Result<BTreeMap<String, String>, String> {
    let (mut fence, mut active) = (None, ActiveMarkdown::default());
    let mut canonical = false;
    let mut blocks = 0;
    let mut values = BTreeMap::new();
    for raw in section.lines() {
        let line = active.line(raw, fence.is_some())?;
        let prior = fence;
        if transition(&mut fence, &line) {
            if prior.is_none() && marker_char(fence) == Some('`') && line.trim() == "```text" {
                canonical = true;
                blocks += 1;
            } else if marker_char(prior) == Some('`') && fence.is_none() && canonical {
                canonical = false;
            }
            continue;
        }
        if canonical {
            if line.trim().is_empty() {
                continue;
            }
            let (key, value) = line.split_once(" = ").ok_or("malformed assignment")?;
            if key.is_empty()
                || value.is_empty()
                || values.insert(key.into(), value.into()).is_some()
            {
                return Err("duplicate or malformed assignment".into());
            }
        } else if fence.is_none() && line.contains(" = ") {
            return Err("assignment outside canonical block".into());
        }
    }
    active.finish()?;
    if fence.is_some() || canonical || blocks != 1 {
        return Err("missing or malformed assignment block".into());
    }
    Ok(values)
}

#[derive(Clone, Copy, PartialEq)]
struct Fence {
    marker: char,
    length: usize,
}

fn transition(fence: &mut Option<Fence>, line: &str) -> bool {
    match *fence {
        None => {
            let Some(open) = opening_marker(line) else {
                return false;
            };
            *fence = Some(open);
            true
        }
        Some(open) => {
            let Some(close) = closing_marker(line) else {
                return false;
            };
            if open.marker == close.marker && close.length >= open.length {
                *fence = None;
                true
            } else {
                false
            }
        }
    }
}

fn marker_char(fence: Option<Fence>) -> Option<char> {
    fence.map(|fence| fence.marker)
}

fn opening_marker(line: &str) -> Option<Fence> {
    marker(line).map(|(fence, _)| fence)
}

fn closing_marker(line: &str) -> Option<Fence> {
    let (fence, suffix) = marker(line)?;
    suffix.trim().is_empty().then_some(fence)
}

fn marker(line: &str) -> Option<(Fence, &str)> {
    let indentation = line.len() - line.trim_start_matches(' ').len();
    if indentation > 3 {
        return None;
    }
    let trimmed = &line[indentation..];
    let marker = trimmed.chars().next()?;
    let length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (matches!(marker, '`' | '~') && length >= 3)
        .then_some((Fence { marker, length }, &trimmed[length..]))
}

fn balanced(fence: Option<Fence>) -> Result<(), String> {
    fence
        .is_none()
        .then_some(())
        .ok_or_else(|| "unbalanced fence".into())
}

fn separator(row: &str) -> Result<bool, String> {
    let cells = cells(row)?;
    Ok(cells.len() == 3
        && cells.iter().all(|cell| {
            let marker = cell.trim_matches(':');
            marker.len() >= 3 && marker.chars().all(|character| character == '-')
        }))
}

fn workflow_row(line: &str) -> Option<&str> {
    let indentation = line.len() - line.trim_start_matches(' ').len();
    (indentation <= 3)
        .then_some(&line[indentation..])
        .filter(|row| row.starts_with('|'))
}

fn cells(row: &str) -> Result<Vec<&str>, String> {
    let row = row.trim();
    if !row.starts_with('|') || !row.ends_with('|') {
        return Err("malformed table row".into());
    }
    Ok(row[1..row.len() - 1].split('|').map(str::trim).collect())
}
