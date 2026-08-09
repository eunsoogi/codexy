use std::collections::BTreeMap;

pub(in crate::support) struct Table {
    pub(super) start: usize,
    end: usize,
    columns: usize,
    rows: Vec<Vec<Cell>>,
}

#[derive(Default)]
struct Cell {
    text: String,
    code: Option<String>,
    literal: bool,
}

pub(super) struct TableBuilder {
    start: usize,
    columns: usize,
    rows: Vec<Vec<Cell>>,
    row: Option<Vec<Cell>>,
    cell: Option<Cell>,
}

impl TableBuilder {
    pub(super) fn new(start: usize, columns: usize) -> Self {
        Self {
            start,
            columns,
            rows: Vec::new(),
            row: None,
            cell: None,
        }
    }

    pub(super) fn row(&mut self) {
        self.row = Some(Vec::new());
    }

    pub(super) fn cell(&mut self) {
        self.cell = Some(Cell {
            literal: true,
            ..Cell::default()
        });
    }

    pub(super) fn text(&mut self, text: &str) {
        if let Some(cell) = &mut self.cell {
            cell.text.push_str(text);
        }
    }

    pub(super) fn code(&mut self, code: &str) {
        if let Some(cell) = &mut self.cell {
            if cell.code.replace(code.into()).is_some() {
                cell.literal = false;
            }
        }
    }

    pub(super) fn mark(&mut self) {
        if let Some(cell) = &mut self.cell {
            cell.literal = false;
        }
    }

    pub(super) fn end_cell(&mut self) {
        if let (Some(row), Some(cell)) = (&mut self.row, self.cell.take()) {
            row.push(cell);
        }
    }

    pub(super) fn end_row(&mut self) {
        if let Some(row) = self.row.take() {
            self.rows.push(row);
        }
    }

    pub(super) fn finish(self, end: usize) -> Table {
        Table {
            start: self.start,
            end,
            columns: self.columns,
            rows: self.rows,
        }
    }
}

impl Table {
    pub(super) fn workflow_rows(&self, source: &str) -> Result<BTreeMap<String, String>, String> {
        if !valid_source(&source[self.start..self.end]) {
            return Err("invalid workflow table separator or indentation".into());
        }
        if self.columns != 3 || self.rows.len() < 3 || self.rows.iter().any(|row| row.len() != 3) {
            return Err("invalid workflow table header".into());
        }
        if !plain(&self.rows[0][0], "Current workflow")
            || !plain(&self.rows[0][1], "Disposition")
            || !plain(&self.rows[0][2], "Contract role")
        {
            return Err("invalid workflow table header".into());
        }
        let mut workflows = BTreeMap::new();
        for row in &self.rows[1..] {
            let Some(name) = row[0]
                .code
                .as_deref()
                .filter(|_| row[0].literal && row[0].text.is_empty())
            else {
                return Err("workflow name must be code-formatted".into());
            };
            if !plain(&row[1], "Keep") && !plain(&row[1], "Merge") && !plain(&row[1], "Remove") {
                return Err("duplicate or invalid workflow disposition".into());
            }
            if !row[2].literal
                || row[2].text.is_empty()
                || workflows.insert(name.into(), row[1].text.clone()).is_some()
            {
                return Err("duplicate or invalid workflow disposition".into());
            }
        }
        Ok(workflows)
    }
}

fn plain(cell: &Cell, expected: &str) -> bool {
    cell.literal && cell.code.is_none() && cell.text == expected
}

fn valid_source(table: &str) -> bool {
    let lines: Vec<_> = table.lines().collect();
    lines.len() >= 3
        && lines
            .iter()
            .all(|line| line.len() - line.trim_start_matches(' ').len() <= 3)
        && separator(lines[1])
}

fn separator(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('|')
        && line.ends_with('|')
        && line[1..line.len() - 1]
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>()
            .as_slice()
            .iter()
            .all(|cell| {
                let marker = cell.trim_matches(':');
                marker.len() >= 3 && marker.bytes().all(|byte| byte == b'-')
            })
        && line[1..line.len() - 1].split('|').count() == 3
}
