use super::child_lane_owner_decision::is_supported_owner_decision;

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
    pub(super) owner: String,
    pub(super) canonical: bool,
}

impl<'a> ClassificationEvidence<'a> {
    pub(super) fn parse(source: &'a str) -> Self {
        let lines = source.lines().map(str::trim).collect::<Vec<_>>();
        let tables = tables(&lines);
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
struct TableRow {
    cells: Vec<String>,
    prefixed: bool,
}

fn tables(lines: &[&str]) -> Vec<ClassificationTable> {
    let mut tables = Vec::new();
    let mut start = 0;
    let mut fenced = false;
    while start < lines.len() {
        if lines[start].starts_with("```") {
            fenced = !fenced;
            start += 1;
            continue;
        }
        if fenced {
            start += 1;
            continue;
        }
        let Some(first) = table_row(lines[start]) else {
            start += 1;
            continue;
        };
        let mut rows = vec![first];
        let mut end = start + 1;
        while end < lines.len() {
            let Some(row) = table_row(lines[end]) else {
                break;
            };
            rows.push(row);
            end += 1;
        }
        if let Some(table) = classification_table(start, end - 1, &rows) {
            tables.push(table);
        }
        start = end;
    }
    tables
}

fn table_row(line: &str) -> Option<TableRow> {
    let (line, prefixed) = without_list_prefix(line);
    let line = line.trim();
    line.contains('|').then(|| TableRow {
        cells: line
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_ascii_lowercase())
            .collect(),
        prefixed,
    })
}

fn without_list_prefix(line: &str) -> (&str, bool) {
    let line = line.trim_start();
    if let Some(rest) = line
        .strip_prefix("- [ ] ")
        .or_else(|| line.strip_prefix("- [x] "))
        .or_else(|| line.strip_prefix("+ "))
        .or_else(|| line.strip_prefix("- "))
    {
        return (rest, true);
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && line.as_bytes().get(digits) == Some(&b'.') {
        return (&line[digits + 1..], true);
    }
    (line, false)
}

fn classification_table(
    start: usize,
    end: usize,
    rows: &[TableRow],
) -> Option<ClassificationTable> {
    let header = rows.first()?;
    let fields = rows
        .iter()
        .filter(|row| !separator(&row.cells))
        .collect::<Vec<_>>();
    let owner = fields
        .iter()
        .skip(1)
        .find_map(|row| (row.cells.first()? == "owner decision").then_some(row.cells.get(1)?))
        .cloned()
        .unwrap_or_default();
    let classification_shaped = header
        .cells
        .first()
        .is_some_and(|cell| cell.contains("task classification"))
        || fields.iter().skip(1).any(|row| {
            row.cells.first().is_some_and(|key| key == "owner decision")
                && fields.iter().skip(1).any(|other| {
                    other.cells.first().is_some_and(|key| {
                        key != "owner decision" && FIELDS.contains(&key.as_str())
                    })
                })
        });
    classification_shaped.then_some(ClassificationTable {
        start,
        end,
        owner: owner.clone(),
        canonical: !rows.iter().any(|row| row.prefixed)
            && matches!(header.cells.as_slice(), [first, second] if first == "task classification" && second == "decision")
            && rows.get(1).is_some_and(|row| valid_separator(&row.cells))
            && fields.len() == FIELDS.len() + 1
            && is_supported_owner_decision(&owner)
            && !has_multiple_owner_tokens(&owner)
            && fields.iter().skip(1).zip(FIELDS).all(|(row, field)| {
                matches!(row.cells.as_slice(), [key, value] if key == field && !value.is_empty())
            }),
    })
}

fn separator(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            !cell.is_empty() && cell.bytes().all(|byte| matches!(byte, b'-' | b':' | b' '))
        })
}

fn valid_separator(cells: &[String]) -> bool {
    separator(cells)
        && cells.iter().all(|cell| {
            cell.trim_matches(':')
                .bytes()
                .filter(|byte| *byte == b'-')
                .count()
                >= 3
        })
}

fn has_multiple_owner_tokens(owner: &str) -> bool {
    [
        "child-owned",
        "current-thread-owned",
        "parent-owned",
        "external/human-owned",
    ]
    .into_iter()
    .filter(|token| affirmative_token(owner, token))
    .count()
        > 1
}

fn affirmative_token(owner: &str, token: &str) -> bool {
    owner.match_indices(token).any(|(index, _)| {
        !owner[..index]
            .rsplit(|character| matches!(character, ',' | ';' | '.'))
            .next()
            .is_some_and(|clause| {
                clause
                    .split(|character: char| !character.is_ascii_alphabetic())
                    .any(|word| matches!(word, "not" | "no" | "without"))
            })
    })
}
