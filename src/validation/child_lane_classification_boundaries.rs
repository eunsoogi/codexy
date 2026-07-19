use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};
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

pub(super) fn next_lane_boundary(lines: &[&str], index: usize) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(index + 1)
        .find(|(index, _)| is_lane_boundary(lines, *index))
        .map_or(lines.len(), |(index, _)| index)
}

fn is_lane_boundary(lines: &[&str], index: usize) -> bool {
    let line = metadata_key(trimmed_value(lines[index]));
    "pr:|pull request:|review response:|maintainer reassignment:"
        .split('|')
        .any(|marker| line.starts_with(marker))
        || is_ownership_boundary(lines[index])
}

pub(super) fn is_ownership_boundary(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        "owner|child owner|lane owner|lane ownership|owner decision|ownership|pr ownership|pull request ownership"
            .split('|')
            .any(|boundary| metadata_key(key) == boundary)
    })
}

pub(super) fn is_legacy_ownership_boundary(line: &str) -> bool {
    let key = line
        .split_once(':')
        .map_or("", |(key, _)| metadata_key(key));
    "owner decision|ownership|lane ownership|pr ownership|pull request ownership"
        .split('|')
        .any(|boundary| key == boundary)
        || field_value(line, "owner").is_some_and(is_parent_owned_value)
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

pub(super) fn owner_at(tables: &[ClassificationTable], index: usize) -> Option<&str> {
    tables
        .iter()
        .find(|table| table.start == index && table.canonical)
        .map(|table| table.owner.as_str())
}

pub(super) fn child_table_owns_handoff_pr(
    tables: &[ClassificationTable],
    lines: &[&str],
    pr_index: usize,
) -> bool {
    classification_owner_before(lines, tables, pr_index)
        .is_some_and(is_child_delegation_owner_decision)
}

pub(super) fn table_ownership_boundary(
    tables: &[ClassificationTable],
    lines: &[&str],
    index: usize,
) -> bool {
    is_ownership_boundary(lines[index])
        && classification_owner_before(lines, tables, index).is_some()
}

pub(super) fn child_table_ownership_boundary(
    tables: &[ClassificationTable],
    lines: &[&str],
    index: usize,
) -> bool {
    table_ownership_boundary(tables, lines, index)
        && lines[index].split_once(':').is_some_and(|(key, value)| {
            let key = metadata_key(key);
            is_child_delegation_owner_decision(value)
                || (key == "child owner"
                    && !value.is_empty()
                    && !value.starts_with("external/human-owned")
                    && !is_parent_owned_value(value)
                    && !value.starts_with("not ")
                    && !value.starts_with("without ")
                    && !matches!(value, "no" | "none" | "false" | "missing" | "absent"))
        })
}

pub(super) fn child_candidate_requires_guard(
    tables: &[ClassificationTable],
    lines: &[&str],
    index: usize,
) -> bool {
    tables.iter().any(|table| {
        !table.canonical
            && table.start < index
            && is_child_delegation_owner_decision(&table.owner)
            && (table.end >= index
                || (table.end + 1..index).all(|line| {
                    !is_lane_boundary(lines, line)
                        && !tables.iter().any(|table| table.start == line)
                }))
    })
}

pub(super) fn classification_owner_before<'a>(
    lines: &[&str],
    tables: &'a [ClassificationTable],
    index: usize,
) -> Option<&'a str> {
    let lane_start = current_lane_start(lines, index);
    let complete = tables
        .iter()
        .filter(|table| {
            table.canonical
                && table.end < index
                && (table_in_lane(table, lane_start, lines)
                    || table_handoff_reaches(table, lines, index))
        })
        .collect::<Vec<_>>();
    (complete.len() == 1).then(|| complete[0].owner.as_str())
}

fn table_handoff_reaches(table: &ClassificationTable, lines: &[&str], index: usize) -> bool {
    let handoff = &lines[table.end + 1..index];
    handoff.first().is_some_and(|line| line.is_empty())
        && handoff.iter().all(|line| {
            line.is_empty()
                || line.split_once(':').is_some_and(|(key, _)| {
                    matches!(
                        metadata_key(key),
                        "issue" | "branch" | "worktree path" | "pr"
                    )
                })
        })
}

fn table_in_lane(table: &ClassificationTable, lane_start: usize, lines: &[&str]) -> bool {
    table.start >= lane_start
        || (table.end < lane_start
            && lines[table.end + 1..lane_start]
                .iter()
                .all(|line| line.is_empty()))
}

fn classification_owner(rows: &[Vec<String>]) -> Option<(String, bool)> {
    matches!(rows.first()?.as_slice(), [key, _] if key.trim().eq_ignore_ascii_case(HEADER[0]))
        .then_some(())?;
    let mut owner = String::new();
    for row in rows.iter().skip(1) {
        let [key, value] = row.as_slice() else {
            break;
        };
        if key.trim().eq_ignore_ascii_case("owner decision") {
            owner = value.to_ascii_lowercase();
        }
    }
    let canonical = rows.first().is_some_and(|header| {
        matches!(header.as_slice(), [first, second] if first.trim().eq_ignore_ascii_case(HEADER[0]) && second.trim().eq_ignore_ascii_case(HEADER[1]))
    })
        && rows.len() == FIELDS.len() + 1
        && rows.iter().skip(1).zip(FIELDS).all(|(row, field)| {
            matches!(row.as_slice(), [key, value] if key.trim().eq_ignore_ascii_case(field) && !value.trim().is_empty())
        });
    Some((owner, canonical))
}

fn line_at(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
}
