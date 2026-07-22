use super::child_lane_classification_fields::ClassificationFields;
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

#[derive(Default)]
enum GfmClassificationTableState {
    #[default]
    Neutral,
    Header,
    Candidate,
    CandidateUnknown,
    Classification,
    ClassificationHeader,
    ClassificationUnknown,
}

pub(super) enum GfmClassificationTableEvent<'a> {
    Ignore,
    Record(&'a str, &'a str),
    ReplaceAndRecord(&'a str, &'a str),
    Invalidate,
    NotGfm,
}

impl GfmClassificationTable {
    pub(super) fn consume<'a>(&mut self, line: &'a str) -> GfmClassificationTableEvent<'a> {
        match &mut self.state {
            GfmClassificationTableState::Neutral => classification_table_row(line)
                .is_some()
                .then(|| {
                    self.state = GfmClassificationTableState::Header;
                    GfmClassificationTableEvent::Ignore
                })
                .unwrap_or(GfmClassificationTableEvent::NotGfm),
            GfmClassificationTableState::Header => {
                self.state = if is_table_separator(line) {
                    GfmClassificationTableState::Candidate
                } else {
                    GfmClassificationTableState::Neutral
                };
                GfmClassificationTableEvent::NotGfm
            }
            GfmClassificationTableState::Candidate => match classification_table_row(line) {
                Some((key, value)) if Self::is_classification_key(key) => {
                    self.state = GfmClassificationTableState::Classification;
                    GfmClassificationTableEvent::ReplaceAndRecord(key, value)
                }
                Some(_) => {
                    self.state = GfmClassificationTableState::CandidateUnknown;
                    GfmClassificationTableEvent::Ignore
                }
                None if malformed_classification_row(line) => self.invalidate(),
                None => GfmClassificationTableEvent::NotGfm,
            },
            GfmClassificationTableState::CandidateUnknown => {
                if is_table_separator(line) {
                    self.state = GfmClassificationTableState::Candidate;
                    return GfmClassificationTableEvent::Ignore;
                }
                match classification_table_row(line) {
                    Some((key, _)) if Self::is_classification_key(key) => self.invalidate(),
                    Some(_) => GfmClassificationTableEvent::Ignore,
                    None if malformed_classification_row(line) => self.invalidate(),
                    None => {
                        self.state = GfmClassificationTableState::Neutral;
                        GfmClassificationTableEvent::NotGfm
                    }
                }
            }
            GfmClassificationTableState::Classification => match classification_table_row(line) {
                Some((key, value)) if Self::is_classification_key(key) => {
                    GfmClassificationTableEvent::Record(key, value)
                }
                Some(("field", "value")) => {
                    self.state = GfmClassificationTableState::ClassificationHeader;
                    GfmClassificationTableEvent::Ignore
                }
                Some(_) => {
                    self.state = GfmClassificationTableState::ClassificationUnknown;
                    GfmClassificationTableEvent::Ignore
                }
                None if line.starts_with('|') => self.invalidate(),
                None => {
                    self.state = GfmClassificationTableState::Neutral;
                    GfmClassificationTableEvent::NotGfm
                }
            },
            GfmClassificationTableState::ClassificationHeader => {
                self.state = if is_table_separator(line) {
                    GfmClassificationTableState::Candidate
                } else {
                    GfmClassificationTableState::Neutral
                };
                GfmClassificationTableEvent::NotGfm
            }
            GfmClassificationTableState::ClassificationUnknown => {
                if is_table_separator(line) {
                    self.state = GfmClassificationTableState::Candidate;
                    GfmClassificationTableEvent::Ignore
                } else {
                    self.invalidate()
                }
            }
        }
    }

    fn invalidate(&mut self) -> GfmClassificationTableEvent<'static> {
        self.state = GfmClassificationTableState::Neutral;
        GfmClassificationTableEvent::Invalidate
    }

    fn is_classification_key(key: &str) -> bool {
        ClassificationFields::records_key(metadata_key(key))
    }
}

fn malformed_classification_row(line: &str) -> bool {
    line.starts_with('|')
        && line
            .strip_prefix('|')
            .and_then(|line| line.split_once('|'))
            .is_some_and(|(key, _)| ClassificationFields::records_key(metadata_key(key)))
}
