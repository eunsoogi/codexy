use super::child_lane_owner_decision::is_child_delegation_owner_decision;
use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

const HEADER: [&str; 2] = ["task classification", "decision"];
const FIELDS: [&str; 8] = [
    "lane type",
    "secondary surfaces",
    "owner decision",
    "atomic scope",
    "required skills",
    "required tools/evidence",
    "first allowed action",
    "stop/blocker",
];

#[derive(Debug)]
pub(super) struct ClassificationTable {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) owner: String,
}

pub(super) fn current_lane_start(lines: &[&str], setup_index: usize) -> usize {
    (0..setup_index)
        .rev()
        .find(|index| is_lane_boundary(lines, *index))
        .map_or(0, |index| index + 1)
}

fn is_lane_boundary(lines: &[&str], index: usize) -> bool {
    let line = metadata_key(trimmed_value(lines[index]));
    if "pr:|pull request:"
        .split('|')
        .any(|marker| line.starts_with(marker))
    {
        return true;
    }
    if line.starts_with("lane ownership:") {
        return !is_inside_task_classification(lines, index);
    }
    is_owner_metadata(line) && !is_inside_task_classification(lines, index)
}

fn is_owner_metadata(line: &str) -> bool {
    "owner:|child owner:|lane owner:|owner decision:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_inside_task_classification(lines: &[&str], index: usize) -> bool {
    for line in lines
        .iter()
        .take(index)
        .rev()
        .map(|line| metadata_key(trimmed_value(line)))
    {
        if line.is_empty() {
            continue;
        }
        if line == "task classification:" {
            return true;
        }
        if is_lane_boundary_terminator(line) || is_hard_lane_boundary(line) {
            return false;
        }
        if !is_task_classification_field(line) {
            return false;
        }
    }
    false
}

fn is_hard_lane_boundary(line: &str) -> bool {
    "pr:|pull request:|lane ownership:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_lane_boundary_terminator(line: &str) -> bool {
    "review response:|maintainer reassignment:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_task_classification_field(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        matches!(
            super::child_lane_ownership_phrases::metadata_key(key),
            "lane type"
                | "secondary surfaces"
                | "owner decision"
                | "atomic scope"
                | "required skills"
                | "required tools/evidence"
                | "required tools"
                | "required evidence"
                | "first allowed action"
                | "stop/blocker"
                | "stop blocker"
                | "blocker"
        )
    })
}

pub(super) fn classifications(source: &str) -> Vec<ClassificationTable> {
    let mut tables = Vec::new();
    let mut current = None::<(usize, Vec<Vec<String>>)>;
    let mut row = None::<Vec<String>>;
    let mut cell = None::<String>;
    for (event, range) in Parser::new_ext(source, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Table(_)) => {
                current = Some((line_at(source, range.start), Vec::new()))
            }
            Event::Start(Tag::TableHead | Tag::TableRow) => row = Some(Vec::new()),
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::Text(text) | Event::Code(text) => {
                if let Some(cell) = &mut cell {
                    cell.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(cell) = &mut cell {
                    cell.push(' ');
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let (Some(row), Some(cell)) = (&mut row, cell.take()) {
                    row.push(cell);
                }
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                if let (Some((_, rows)), Some(row)) = (&mut current, row.take()) {
                    rows.push(row);
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some((start, rows)) = current.take() {
                    if let Some(owner) = canonical_owner(&rows) {
                        tables.push(ClassificationTable {
                            start,
                            end: line_at(source, range.end.saturating_sub(1)),
                            owner,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    tables
}

pub(super) fn is_classification_key(key: &str) -> bool {
    matches!(
        key,
        "lane type"
            | "secondary surfaces"
            | "owner decision"
            | "atomic scope"
            | "required skills"
            | "required tools/evidence"
            | "required tools"
            | "required evidence"
            | "first allowed action"
            | "stop/blocker"
            | "stop blocker"
            | "blocker"
    )
}

pub(super) fn owner_at(tables: &[ClassificationTable], index: usize) -> Option<&str> {
    tables
        .iter()
        .find(|table| table.start == index)
        .map(|table| table.owner.as_str())
}

pub(super) fn rendered_child_context_applies(
    lines: &[&str],
    tables: &[ClassificationTable],
    setup_index: usize,
) -> bool {
    let Some(table) = tables.iter().rev().find(|table| table.end < setup_index) else {
        return false;
    };
    is_child_delegation_owner_decision(&table.owner)
        && !lines[table.end + 1..setup_index].iter().any(|line| {
            let line = metadata_key(trimmed_value(line));
            line.starts_with("lane ownership:") || line.starts_with("owner decision:")
        })
}

fn canonical_owner(rows: &[Vec<String>]) -> Option<String> {
    (rows.len() == FIELDS.len() + 1 && cells_match(&rows[0], &HEADER)).then_some(())?;
    let mut owner = None;
    for (row, field) in rows.iter().skip(1).zip(FIELDS) {
        let [key, value] = row.as_slice() else {
            return None;
        };
        if !key.trim().eq_ignore_ascii_case(field) || value.trim().is_empty() {
            return None;
        }
        if field == "owner decision" {
            owner = Some(value.to_ascii_lowercase());
        }
    }
    owner
}

fn cells_match(cells: &[String], expected: &[&str]) -> bool {
    cells.len() == expected.len()
        && cells
            .iter()
            .zip(expected)
            .all(|(cell, expected)| cell.trim().eq_ignore_ascii_case(expected))
}

fn line_at(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
}
