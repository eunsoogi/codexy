use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::child_lane_owner_decision::OwnerDecision;
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
pub(super) struct ClassificationEvidence<'a> {
    lines: Vec<&'a str>,
    tables: Vec<ClassificationTable>,
}
#[derive(Debug)]
pub(super) struct ClassificationTable {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) owner: Option<OwnerDecision>,
    pub(super) canonical: bool,
}
impl<'a> ClassificationEvidence<'a> {
    pub(super) fn parse(source: &'a str) -> Self {
        let raw_lines = source_lines(source);
        let lines = raw_lines.iter().map(|(line, _)| line.trim()).collect();
        let (rendered, excluded) = rendered_tables(source);
        let mut tables = rendered
            .iter()
            .filter_map(|table| classification_table(table, source))
            .collect::<Vec<_>>();
        let blocked_spans = rendered
            .iter()
            .map(|table| table.span.clone())
            .chain(excluded)
            .collect::<Vec<_>>();
        tables.extend(invalid_candidates(&raw_lines, &blocked_spans));
        Self { lines, tables }
    }
    pub(super) fn lines(&self) -> &[&'a str] {
        &self.lines
    }
    pub(super) fn tables(&self) -> &[ClassificationTable] {
        &self.tables
    }
}
#[derive(Debug)]
struct RenderedTable {
    span: Range<usize>,
    rows: Vec<Vec<String>>,
}
fn rendered_tables(source: &str) -> (Vec<RenderedTable>, Vec<Range<usize>>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let (mut tables, mut excluded, mut open) = (Vec::new(), Vec::new(), None);
    for (event, span) in Parser::new_ext(source, options).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) | Event::Start(Tag::HtmlBlock) => excluded.push(span),
            Event::Html(_) => excluded.push(span),
            Event::Start(Tag::Table(_)) => {
                open = Some(RenderedTable {
                    span,
                    rows: Vec::new(),
                })
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                if let Some(table) = &mut open {
                    table.rows.push(Vec::new());
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(table) = &mut open {
                    if table.rows.is_empty() {
                        table.rows.push(Vec::new());
                    }
                    table
                        .rows
                        .last_mut()
                        .expect("table row")
                        .push(String::new());
                }
            }
            Event::Text(text) | Event::Code(text) => append_cell(&mut open, &text),
            Event::SoftBreak | Event::HardBreak => append_cell(&mut open, " "),
            Event::End(TagEnd::Table) => {
                if let Some(table) = open.take() {
                    tables.push(table);
                }
            }
            _ => {}
        }
    }
    (tables, excluded)
}
fn append_cell(table: &mut Option<RenderedTable>, text: &str) {
    if let Some(cell) = table
        .as_mut()
        .and_then(|table| table.rows.last_mut())
        .and_then(|row| row.last_mut())
    {
        cell.push_str(text);
    }
}
fn classification_table(table: &RenderedTable, source: &str) -> Option<ClassificationTable> {
    let rows = table
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| cell.trim().to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let header = rows.first()?;
    let owner = rows
        .iter()
        .skip(1)
        .find_map(|row| (row.first()? == "owner decision").then_some(row.get(1)?))
        .cloned()
        .unwrap_or_default();
    let owner = OwnerDecision::parse(&owner);
    let shaped = header
        .first()
        .is_some_and(|cell| cell.contains("task classification"))
        || rows.iter().skip(1).any(|row| {
            row.first().is_some_and(|key| key == "owner decision")
                && rows.iter().skip(1).any(|other| {
                    other.first().is_some_and(|key| {
                        key != "owner decision" && FIELDS.contains(&key.as_str())
                    })
                })
        });
    shaped.then_some(ClassificationTable {
        start: source[..table.span.start].matches('\n').count(),
        end: source[..table.span.end.saturating_sub(1)].matches('\n').count(),
        owner,
        canonical: matches!(header.as_slice(), [first, second] if first == "task classification" && second == "decision")
            && valid_delimiter(source, &table.span, header.len())
            && rows.len() == FIELDS.len() + 1
            && owner.is_some()
            && rows.iter().skip(1).zip(FIELDS).all(|(row, field)| {
                matches!(row.as_slice(), [key, value] if key == field && !value.is_empty())
            }),
    })
}
fn valid_delimiter(source: &str, span: &Range<usize>, columns: usize) -> bool {
    source[span.clone()].lines().nth(1).is_some_and(|line| {
        let mut cells = line.trim().trim_matches('|').split('|').map(str::trim);
        cells.clone().count() == columns
            && cells.all(|cell| {
                cell.trim_matches(':')
                    .bytes()
                    .filter(|byte| *byte == b'-')
                    .count()
                    >= 3
                    && cell.bytes().all(|byte| matches!(byte, b'-' | b':' | b' '))
            })
    })
}
fn source_lines(source: &str) -> Vec<(&str, Range<usize>)> {
    let mut offset = 0;
    source
        .split_inclusive('\n')
        .map(|raw| {
            let line = raw
                .strip_suffix("\r\n")
                .or_else(|| raw.strip_suffix('\n'))
                .unwrap_or(raw);
            let span = offset..offset + line.len();
            offset += raw.len();
            (line, span)
        })
        .collect()
}
fn invalid_candidates(
    lines: &[(&str, Range<usize>)],
    blocked_spans: &[Range<usize>],
) -> Vec<ClassificationTable> {
    let (mut tables, mut start) = (Vec::new(), 0);
    while start < lines.len() {
        if !lines[start].0.contains('|') || covered(lines[start].1.clone(), blocked_spans) {
            start += 1;
            continue;
        }
        let first = start;
        while start < lines.len() {
            if !lines[start].0.contains('|') || covered(lines[start].1.clone(), blocked_spans) {
                break;
            }
            start += 1;
        }
        if recognizable(&lines[first..start]) {
            tables.push(ClassificationTable {
                start: first,
                end: start - 1,
                owner: None,
                canonical: false,
            });
        }
    }
    tables
}
fn covered(span: Range<usize>, spans: &[Range<usize>]) -> bool {
    spans
        .iter()
        .any(|other| span.start < other.end && other.start < span.end)
}
fn recognizable(lines: &[(&str, Range<usize>)]) -> bool {
    let text = lines
        .iter()
        .fold(String::new(), |mut text, (line, _)| {
            text.push_str(line);
            text.push('\n');
            text
        })
        .to_ascii_lowercase();
    text.contains("task classification")
        || (text.contains("owner decision")
            && FIELDS
                .iter()
                .any(|field| *field != "owner decision" && text.contains(field)))
}
