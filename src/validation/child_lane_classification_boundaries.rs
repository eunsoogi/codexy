use super::child_lane_owner_decision::is_child_delegation_owner_decision;
use super::child_lane_ownership_phrases::{field_value, metadata_key, trimmed_value};
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
    canonical: bool,
}

pub(super) fn current_lane_start(lines: &[&str], setup_index: usize) -> usize {
    (0..setup_index)
        .rev()
        .find(|index| is_lane_boundary(lines, *index))
        .map_or(0, |index| index + 1)
}

fn is_lane_boundary(lines: &[&str], index: usize) -> bool {
    let line = metadata_key(trimmed_value(lines[index]));
    if "pr:|pull request:|review response:|maintainer reassignment:"
        .split('|')
        .any(|marker| line.starts_with(marker))
    {
        return true;
    }
    line.starts_with("lane ownership:") || line.starts_with("owner decision:")
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
                    if let Some((owner, canonical)) = classification_owner(&rows) {
                        tables.push(ClassificationTable {
                            start,
                            end: start + rows.len(),
                            owner,
                            canonical,
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
        .find(|table| table.start == index && table.canonical)
        .map(|table| table.owner.as_str())
}

pub(super) fn rendered_child_context_applies(
    lines: &[&str],
    tables: &[ClassificationTable],
    setup_index: usize,
) -> bool {
    let lane_start = current_lane_start(lines, setup_index);
    let context_start = lines[..setup_index]
        .iter()
        .rposition(|line| {
            let line = metadata_key(trimmed_value(line));
            line.starts_with("pr:")
                || line.starts_with("pull request:")
                || line.starts_with("review response:")
                || line.starts_with("maintainer reassignment:")
        })
        .map_or(0, |index| index + 1);
    let lane_end = lines
        .iter()
        .enumerate()
        .skip(setup_index + 1)
        .find(|(index, _)| is_lane_boundary(lines, *index))
        .map_or(lines.len(), |(index, _)| index);
    let tables = tables
        .iter()
        .filter(|table| table_in_lane(table, lane_start, lines) && table.start < lane_end)
        .collect::<Vec<_>>();
    tables
        .iter()
        .any(|table| is_child_delegation_owner_decision(&table.owner) || table.owner.is_empty())
        || lines[context_start..lane_end]
            .iter()
            .take_while(|line| !line.starts_with("pr:") && !line.starts_with("pull request:"))
            .any(|line| is_explicit_child_context(line))
        || lines[context_start..lane_end]
            .iter()
            .any(|line| metadata_key(trimmed_value(line)) == "task classification:")
}

fn is_explicit_child_context(line: &str) -> bool {
    let line = metadata_key(trimmed_value(line));
    matches!(line, "child-owned" | "child-owned lane")
        || field_value(line, "owner decision").is_some_and(is_child_delegation_owner_decision)
        || "lane ownership: child-owned|owner: child-owned|lane owner: child-owned"
            .split('|')
            .any(|marker| line.starts_with(marker))
        || field_value(line, "child owner")
            .is_some_and(|value| !value.is_empty() && !value.contains("none"))
}

pub(super) fn complete_classification_before(
    lines: &[&str],
    tables: &[ClassificationTable],
    setup_index: usize,
) -> Option<usize> {
    let lane_start = current_lane_start(lines, setup_index);
    let complete = tables
        .iter()
        .filter(|table| {
            table.canonical && table_in_lane(table, lane_start, lines) && table.end < setup_index
        })
        .collect::<Vec<_>>();
    (complete.len() == 1 && is_child_delegation_owner_decision(&complete[0].owner))
        .then(|| complete[0].end)
}

fn table_in_lane(table: &ClassificationTable, lane_start: usize, lines: &[&str]) -> bool {
    table.start >= lane_start
        || (table.end + 1 < lane_start
            && lines[table.end + 1..lane_start - 1]
                .iter()
                .all(|line| line.is_empty()))
}

fn classification_owner(rows: &[Vec<String>]) -> Option<(String, bool)> {
    cells_match(rows.first()?, &HEADER).then_some(())?;
    let mut owner = String::new();
    for row in rows.iter().skip(1) {
        let [key, value] = row.as_slice() else {
            break;
        };
        if key.trim().eq_ignore_ascii_case("owner decision") {
            owner = value.to_ascii_lowercase();
        }
    }
    let canonical = rows.len() == FIELDS.len() + 1
        && rows.iter().skip(1).zip(FIELDS).all(|(row, field)| {
            matches!(row.as_slice(), [key, value] if key.trim().eq_ignore_ascii_case(field) && !value.trim().is_empty())
        });
    Some((owner, canonical))
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
