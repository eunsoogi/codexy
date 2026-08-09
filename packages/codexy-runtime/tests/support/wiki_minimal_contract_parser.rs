use std::collections::BTreeMap;

pub(crate) fn section(text: &str, title: &str) -> Result<String, String> {
    let mut fence = None;
    let count = text
        .lines()
        .filter(|line| {
            transition(&mut fence, line);
            fence.is_none() && *line == title
        })
        .count();
    if count != 1 {
        return Err(format!("missing or duplicate section {title}"));
    }
    let level = title
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let mut body = None;
    fence = None;
    for line in text.lines() {
        transition(&mut fence, line);
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
            body.push_str(line);
            body.push('\n');
        }
    }
    body.ok_or_else(|| format!("missing section {title}"))
}

pub(crate) fn workflow_rows(table: &str) -> Result<BTreeMap<String, String>, String> {
    let mut rows = Vec::new();
    let mut fence = None;
    for line in table.lines() {
        transition(&mut fence, line);
        if line.trim_start().starts_with('|') {
            if fence.is_some() {
                return Err("workflow table rows cannot be fenced".into());
            }
            rows.push(line);
        }
    }
    if rows.len() < 3 || cells(rows[0])? != ["Current workflow", "Disposition", "Contract role"] {
        return Err("invalid workflow table header".into());
    }
    if !separator(rows[1])? {
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
    let mut fence = None;
    let mut canonical = false;
    let mut blocks = 0;
    let mut values = BTreeMap::new();
    for line in section.lines() {
        let prior = fence;
        if transition(&mut fence, line) {
            if prior.is_none() && fence == Some('`') && line.trim() == "```text" {
                canonical = true;
                blocks += 1;
            } else if prior == Some('`') && fence.is_none() && canonical {
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
    if fence.is_some() || canonical || blocks != 1 {
        return Err("missing or malformed assignment block".into());
    }
    Ok(values)
}

fn transition(fence: &mut Option<char>, line: &str) -> bool {
    let Some(marker) = marker(line) else {
        return false;
    };
    if fence.is_none() || *fence == Some(marker) {
        *fence = fence.map_or(Some(marker), |_| None);
        true
    } else {
        false
    }
}

fn marker(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    (matches!(marker, '`' | '~')
        && trimmed
            .chars()
            .take_while(|character| *character == marker)
            .count()
            >= 3)
        .then_some(marker)
}

fn separator(row: &str) -> Result<bool, String> {
    let cells = cells(row)?;
    Ok(cells.len() == 3
        && cells.iter().all(|cell| {
            let marker = cell.trim_matches(':');
            marker.len() >= 3 && marker.chars().all(|character| character == '-')
        }))
}

fn cells(row: &str) -> Result<Vec<&str>, String> {
    let row = row.trim();
    if !row.starts_with('|') || !row.ends_with('|') {
        return Err("malformed table row".into());
    }
    Ok(row[1..row.len() - 1].split('|').map(str::trim).collect())
}
