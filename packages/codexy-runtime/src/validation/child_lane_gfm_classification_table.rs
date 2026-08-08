use super::child_lane_classification_schema::ClassificationTableSchema;
use super::child_lane_ownership_phrases::metadata_key;

pub(super) fn classification_table_row(line: &str) -> Option<(&str, &str)> {
    let cells = gfm_table_cells(line)?;
    let [key, value] = cells.as_slice() else {
        return None;
    };
    Some((*key, *value))
}

fn gfm_table_cells(line: &str) -> Option<Vec<&str>> {
    let line = line.strip_prefix('|')?;
    let row = line.strip_suffix('|')?;
    (!is_escaped_pipe(row, row.len())).then_some(())?;
    let mut cells = Vec::new();
    let mut start = 0;
    for (index, _) in row.match_indices('|') {
        if !is_escaped_pipe(row, index) {
            cells.push(row[start..index].trim());
            start = index + 1;
        }
    }
    cells.push(row[start..].trim());
    Some(cells)
}

fn is_escaped_pipe(row: &str, pipe_index: usize) -> bool {
    row[..pipe_index]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count()
        % 2
        == 1
}

enum GfmDelimiterRow {
    Valid,
    Invalid,
    Absent,
}

fn parse_table_separator(line: &str) -> GfmDelimiterRow {
    let Some(cells) = gfm_table_cells(line) else {
        return GfmDelimiterRow::Absent;
    };
    if cells.len() == 2 && cells.iter().all(|cell| is_gfm_delimiter_cell(cell)) {
        return GfmDelimiterRow::Valid;
    }
    if cells.iter().any(|cell| is_delimiter_candidate_cell(cell)) {
        return GfmDelimiterRow::Invalid;
    }
    GfmDelimiterRow::Absent
}

fn is_delimiter_candidate_cell(cell: &str) -> bool {
    cell.is_empty() || matches!(cell.chars().next(), Some('-' | ':'))
}

fn is_gfm_delimiter_cell(cell: &str) -> bool {
    let cell = cell.strip_prefix(':').unwrap_or(cell);
    let cell = cell.strip_suffix(':').unwrap_or(cell);
    cell.len() >= 3 && cell.chars().all(|character| character == '-')
}

#[derive(Default)]
pub(super) struct GfmClassificationTable {
    state: GfmClassificationTableState,
}

#[derive(Default, Clone, Copy)]
enum GfmClassificationTableState {
    #[default]
    Neutral,
    CanonicalHeader,
    Classification {
        next_field: usize,
    },
    Complete,
    SchemaKeyHeader,
    OtherHeader,
}

pub(super) enum GfmClassificationTableEvent<'a> {
    Ignore,
    Replace,
    Record(&'a str, &'a str),
    Invalidate,
    NotGfm,
}

impl GfmClassificationTable {
    pub(super) fn has_unconfirmed_header(&self) -> bool {
        matches!(
            self.state,
            GfmClassificationTableState::SchemaKeyHeader | GfmClassificationTableState::OtherHeader
        )
    }

    pub(super) fn consume<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        match self.state {
            GfmClassificationTableState::Neutral => self.consume_neutral(line),
            GfmClassificationTableState::CanonicalHeader => self.consume_header(line),
            GfmClassificationTableState::Classification { next_field } => {
                self.consume_classification(line, next_field)
            }
            GfmClassificationTableState::Complete => self.consume_complete(line),
            GfmClassificationTableState::SchemaKeyHeader => self.consume_schema_key_header(line),
            GfmClassificationTableState::OtherHeader => self.consume_other_header(line),
        }
    }

    fn consume_neutral<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        match classification_table_row(line) {
            Some((key, value)) if ClassificationTableSchema::has_canonical_header(key, value) => {
                self.state = GfmClassificationTableState::CanonicalHeader;
                GfmClassificationTableEvent::Ignore
            }
            Some(_) => GfmClassificationTableEvent::Ignore,
            None => GfmClassificationTableEvent::NotGfm,
        }
    }

    fn consume_header<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        match parse_table_separator(line) {
            GfmDelimiterRow::Valid => {
                self.state = GfmClassificationTableState::Classification { next_field: 0 };
                GfmClassificationTableEvent::Replace
            }
            GfmDelimiterRow::Invalid => self.invalidate(),
            GfmDelimiterRow::Absent => self.invalidate(),
        }
    }

    fn consume_classification<'a>(
        &mut self,
        line: &'a str,
        next_field: usize,
    ) -> GfmClassificationTableEvent<'a> {
        let Some((key, value)) = classification_table_row(line) else {
            return self.invalidate();
        };
        let key = metadata_key(key);
        if ClassificationTableSchema::accepts(next_field, key, value) {
            self.state = (next_field + 1 == ClassificationTableSchema::field_count())
                .then_some(GfmClassificationTableState::Complete)
                .unwrap_or(GfmClassificationTableState::Classification {
                    next_field: next_field + 1,
                });
            return GfmClassificationTableEvent::Record(key, value);
        }
        self.invalidate()
    }

    fn consume_complete<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        match classification_table_row(line) {
            Some((key, value)) if ClassificationTableSchema::has_canonical_header(key, value) => {
                self.state = GfmClassificationTableState::CanonicalHeader;
                GfmClassificationTableEvent::Ignore
            }
            Some((key, _)) if ClassificationTableSchema::records_key(metadata_key(key)) => {
                self.state = GfmClassificationTableState::SchemaKeyHeader;
                GfmClassificationTableEvent::Ignore
            }
            Some(_) => {
                self.state = GfmClassificationTableState::OtherHeader;
                GfmClassificationTableEvent::Ignore
            }
            None => {
                self.state = GfmClassificationTableState::Neutral;
                GfmClassificationTableEvent::NotGfm
            }
        }
    }

    fn consume_schema_key_header<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        if matches!(parse_table_separator(line), GfmDelimiterRow::Valid) {
            return self.invalidate();
        }
        self.invalidate()
    }

    fn consume_other_header<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        if matches!(parse_table_separator(line), GfmDelimiterRow::Valid) {
            return self.invalidate();
        }
        self.invalidate()
    }

    fn invalidate(&mut self) -> GfmClassificationTableEvent<'static> {
        self.state = GfmClassificationTableState::Neutral;
        GfmClassificationTableEvent::Invalidate
    }
}
