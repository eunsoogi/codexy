use std::collections::BTreeMap;

pub(crate) const WORKFLOWS: &[(&str, &str)] = &[
    ("init", "Keep"),
    ("ingest", "Keep"),
    ("ingest-collection", "Merge"),
    ("collect", "Remove"),
    ("compile", "Keep"),
    ("query", "Keep"),
    ("refresh", "Keep"),
    ("lint", "Keep"),
    ("librarian", "Merge"),
    ("audit", "Merge"),
    ("research", "Merge"),
    ("output", "Merge"),
    ("plan", "Remove"),
    ("project", "Remove"),
    ("inventory", "Remove"),
    ("dataset", "Remove"),
    ("archive", "Remove"),
    ("ll", "Remove"),
    ("assess", "Merge"),
    ("retract", "Merge"),
    ("thesis", "Merge"),
    ("status", "Remove"),
    ("session", "Remove"),
    ("session-capture", "Remove"),
    ("rehydrate", "Remove"),
    ("session-promote", "Merge"),
    ("feedback", "Remove"),
    ("feedback-capture", "Remove"),
    ("feedback-promote", "Merge"),
];

pub(crate) const ASSIGNMENTS: &[(&str, &str)] = &[
    ("query.max_index_files", "3"),
    ("query.max_article_files", "8"),
    ("query.max_index_file_bytes", "4000"),
    ("query.max_article_file_bytes", "4000"),
    ("query.max_total_bytes", "48000"),
    ("freshness.threshold", "70"),
    ("freshness.hot_half_life_days", "30"),
    ("freshness.warm_half_life_days", "90"),
    ("freshness.cold_half_life_days", "365"),
    ("freshness.decay", "25 * 0.5^(age_days / half_life_days)"),
    (
        "freshness.source_age",
        "max(age_days across resolvable sources)",
    ),
    (
        "freshness.source_chain",
        "25 * resolvable_sources / total_sources",
    ),
    (
        "freshness.score",
        "round_half_up(decay(source_age) + decay(verification_age) + decay(compilation_age) + source_chain)",
    ),
    ("freshness.future_date", "0"),
    (
        "freshness.conversation",
        "min(100, 2 * (verification + compilation))",
    ),
];

pub(crate) fn validate_contract(text: &str) -> Result<(), String> {
    let workflows = workflow_rows(&section(text, "## Current workflow disposition")?)?;
    let measures = assignments(&section(text, "### Machine-checkable limits")?)?;
    for title in ["## Essential contract", "## Measurable criteria"] {
        section(text, title)?;
    }
    for title in [
        "### Ingest",
        "### Compile",
        "### Query",
        "### Refresh and provenance",
        "### Bounded context",
        "### Context efficiency",
        "### Traceability",
        "### Freshness",
    ] {
        section(text, title)?;
    }
    if workflows.len() != WORKFLOWS.len() {
        return Err("incomplete workflow inventory".into());
    }
    for (workflow, disposition) in WORKFLOWS {
        if workflows.get(*workflow).map(String::as_str) != Some(*disposition) {
            return Err(format!("invalid disposition for {workflow}"));
        }
    }
    if measures.len() != ASSIGNMENTS.len() {
        return Err("unexpected or incomplete assignment inventory".into());
    }
    for (key, value) in ASSIGNMENTS {
        if measures.get(*key).map(String::as_str) != Some(*value) {
            return Err(format!("invalid assignment {key}"));
        }
    }
    Ok(())
}

fn section(text: &str, title: &str) -> Result<String, String> {
    let occurrences = text
        .lines()
        .scan(false, |fenced, line| {
            if line.trim_start().starts_with("```") {
                *fenced = !*fenced;
            }
            Some(!*fenced && line == title)
        })
        .filter(|matches| *matches)
        .count();
    if occurrences != 1 {
        return Err(format!("missing or duplicate section {title}"));
    }
    let level = title
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let mut body = None;
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if !fenced && line == title {
            body = Some(String::new());
            continue;
        }
        let next_level = line
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if body.is_some()
            && !fenced
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

fn workflow_rows(table: &str) -> Result<BTreeMap<String, String>, String> {
    let mut rows = Vec::new();
    let mut fenced = false;
    for line in table.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if line.trim_start().starts_with('|') {
            if fenced {
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

fn assignments(section: &str) -> Result<BTreeMap<String, String>, String> {
    let mut in_block = false;
    let mut blocks = 0;
    let mut values = BTreeMap::new();
    for line in section.lines() {
        if line.trim_start().starts_with("```") {
            in_block = !in_block;
            blocks += 1;
            continue;
        }
        if !in_block {
            if line.contains(" = ") {
                return Err("assignment outside canonical block".into());
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line.split_once(" = ").ok_or("malformed assignment")?;
        if key.is_empty() || value.is_empty() || values.insert(key.into(), value.into()).is_some() {
            return Err("duplicate or malformed assignment".into());
        }
    }
    if in_block || blocks != 2 {
        return Err("missing or malformed assignment block".into());
    }
    Ok(values)
}

fn separator(row: &str) -> Result<bool, String> {
    Ok(cells(row)?.iter().all(|cell| {
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
