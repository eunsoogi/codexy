use super::child_lane_classification_schema::ClassificationTableSchema;
use super::child_lane_ownership_phrases::metadata_key;

pub(super) fn classification_table_row(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_prefix('|')?;
    let closing_pipe = line.len().checked_sub(1)?;
    (!is_escaped_pipe(line, closing_pipe)).then_some(())?;
    let row = line.strip_suffix('|')?;
    let separator = row
        .match_indices('|')
        .find_map(|(index, _)| (!is_escaped_pipe(row, index)).then_some(index))?;
    let (key, value) = row.split_at(separator);
    let value = &value[1..];
    (!value
        .match_indices('|')
        .any(|(index, _)| !is_escaped_pipe(value, index)))
    .then_some((key.trim(), value.trim()))
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

fn is_table_separator(line: &str) -> bool {
    classification_table_row(line)
        .is_some_and(|(key, value)| is_gfm_delimiter_cell(key) && is_gfm_delimiter_cell(value))
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
    pub(super) fn consume<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        match self.state {
            GfmClassificationTableState::Neutral => self.consume_neutral(line),
            GfmClassificationTableState::CanonicalHeader => self.consume_header(line),
            GfmClassificationTableState::Classification { next_field } => {
                self.consume_classification(line, next_field)
            }
            GfmClassificationTableState::Complete => self.consume_complete(line),
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
        if is_table_separator(line) {
            self.state = GfmClassificationTableState::Classification { next_field: 0 };
            GfmClassificationTableEvent::Replace
        } else {
            self.state = GfmClassificationTableState::Neutral;
            GfmClassificationTableEvent::NotGfm
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
                self.invalidate()
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

    fn consume_other_header<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        if is_table_separator(line) {
            self.state = GfmClassificationTableState::Neutral;
            return GfmClassificationTableEvent::NotGfm;
        }
        match classification_table_row(line) {
            Some((key, _)) if ClassificationTableSchema::records_key(metadata_key(key)) => {
                self.invalidate()
            }
            Some(_) => GfmClassificationTableEvent::Ignore,
            None => {
                self.state = GfmClassificationTableState::Neutral;
                GfmClassificationTableEvent::NotGfm
            }
        }
    }

    fn invalidate(&mut self) -> GfmClassificationTableEvent<'static> {
        self.state = GfmClassificationTableState::Neutral;
        GfmClassificationTableEvent::Invalidate
    }
}
