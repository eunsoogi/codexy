use std::collections::BTreeMap;

use crate::support::{
    wiki_minimal_contract_activity::ActiveMarkdown, wiki_minimal_contract_fence::FenceState,
};

pub(crate) fn section(text: &str, title: &str) -> Result<String, String> {
    let (mut fence, mut active) = (FenceState::default(), ActiveMarkdown::default());
    let mut count = 0;
    for raw in text.lines() {
        let line = active.line(raw, fence.is_fenced())?;
        fence.transition(&line);
        if !fence.is_fenced() && line == title {
            count += 1;
        }
    }
    active.finish()?;
    fence.finish()?;
    if count != 1 {
        return Err(format!("missing or duplicate section {title}"));
    }
    let level = title
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let mut body = None;
    (fence, active) = (FenceState::default(), ActiveMarkdown::default());
    for raw in text.lines() {
        let line = active.line(raw, fence.is_fenced())?;
        fence.transition(&line);
        if !fence.is_fenced() && line == title {
            body = Some(String::new());
            continue;
        }
        let next_level = line
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if body.is_some()
            && !fence.is_fenced()
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
    fence.finish()?;
    body.ok_or_else(|| format!("missing section {title}"))
}

pub(crate) fn workflow_rows(table: &str) -> Result<BTreeMap<String, String>, String> {
    let mut rows: Vec<String> = Vec::new();
    let (mut fence, mut active) = (FenceState::default(), ActiveMarkdown::default());
    let mut table_ended = false;
    for raw in table.lines() {
        let line = active.line(raw, fence.is_fenced())?;
        fence.transition(&line);
        if let Some(row) = workflow_row(&line) {
            if fence.is_fenced() {
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
    fence.finish()?;
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
    let (mut fence, mut active) = (FenceState::default(), ActiveMarkdown::default());
    let mut canonical = false;
    let mut blocks = 0;
    let mut values = BTreeMap::new();
    for raw in section.lines() {
        let line = active.line(raw, fence.is_fenced())?;
        let prior_marker = fence.marker();
        if fence.transition(&line) {
            if prior_marker.is_none() && fence.marker() == Some('`') && line.trim() == "```text" {
                canonical = true;
                blocks += 1;
            } else if prior_marker == Some('`') && !fence.is_fenced() && canonical {
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
        } else if !fence.is_fenced() && line.contains(" = ") {
            return Err("assignment outside canonical block".into());
        }
    }
    active.finish()?;
    if fence.is_fenced() || canonical || blocks != 1 {
        return Err("missing or malformed assignment block".into());
    }
    Ok(values)
}

pub(crate) fn active_link_lines(text: &str) -> Result<Vec<String>, String> {
    let (mut fence, mut active) = (FenceState::default(), ActiveMarkdown::default());
    let mut lines = Vec::new();
    for raw in text.lines() {
        let fenced = fence.is_fenced();
        let line = active.link_line(raw, fenced)?;
        let delimiter = fence.transition(&line);
        if !fenced && !delimiter {
            lines.push(line);
        }
    }
    active.finish()?;
    fence.finish()?;
    Ok(lines)
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
