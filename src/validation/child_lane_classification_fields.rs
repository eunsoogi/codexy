use super::child_lane_classification_authority::LaneAuthority;
use super::child_lane_owner_decision::{
    is_affirmative_child_owned_value, is_affirmative_child_owner_decision,
    is_affirmative_owner_decision_for, is_child_delegation_owner_decision, is_parent_owned_value,
};
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

pub(super) fn is_table_separator(line: &str) -> bool {
    classification_table_row(line)
        .is_some_and(|(key, value)| is_gfm_delimiter_cell(key) && is_gfm_delimiter_cell(value))
}

fn is_gfm_delimiter_cell(cell: &str) -> bool {
    let cell = cell.strip_prefix(':').unwrap_or(cell);
    let cell = cell.strip_suffix(':').unwrap_or(cell);
    cell.len() >= 3 && cell.chars().all(|character| character == '-')
}

#[derive(Clone, Default)]
pub(super) struct ClassificationFields {
    lane_type: bool,
    secondary_surfaces: bool,
    atomic_scope: bool,
    required_skills: bool,
    required_tools: bool,
    first_allowed_action: bool,
    stop_blocker: bool,
    child_display_owner_decision: bool,
    child_owner_decision: bool,
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
    Candidate {
        has_nonclassification_row: bool,
    },
    Classification,
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
        if classification_table_row(line) == Some(("field", "value")) {
            self.state = GfmClassificationTableState::Header;
            return GfmClassificationTableEvent::Ignore;
        }
        match &mut self.state {
            GfmClassificationTableState::Neutral => GfmClassificationTableEvent::NotGfm,
            GfmClassificationTableState::Header => {
                if is_table_separator(line) {
                    self.state = GfmClassificationTableState::Candidate {
                        has_nonclassification_row: false,
                    };
                    GfmClassificationTableEvent::Ignore
                } else {
                    self.state = GfmClassificationTableState::Neutral;
                    malformed_classification_row(line)
                        .then_some(GfmClassificationTableEvent::Invalidate)
                        .unwrap_or(GfmClassificationTableEvent::NotGfm)
                }
            }
            GfmClassificationTableState::Candidate {
                has_nonclassification_row,
            } => match classification_table_row(line) {
                Some((key, value)) if Self::is_classification_key(key) => {
                    if *has_nonclassification_row {
                        self.state = GfmClassificationTableState::Neutral;
                        GfmClassificationTableEvent::Invalidate
                    } else {
                        self.state = GfmClassificationTableState::Classification;
                        GfmClassificationTableEvent::ReplaceAndRecord(key, value)
                    }
                }
                Some(_) => {
                    *has_nonclassification_row = true;
                    GfmClassificationTableEvent::Ignore
                }
                None if malformed_classification_row(line) => {
                    self.state = GfmClassificationTableState::Neutral;
                    GfmClassificationTableEvent::Invalidate
                }
                None => GfmClassificationTableEvent::NotGfm,
            },
            GfmClassificationTableState::Classification => match classification_table_row(line) {
                Some((key, value)) if Self::is_classification_key(key) => {
                    GfmClassificationTableEvent::Record(key, value)
                }
                Some(_) => {
                    self.state = GfmClassificationTableState::Neutral;
                    GfmClassificationTableEvent::Invalidate
                }
                None if line.starts_with('|') => {
                    self.state = GfmClassificationTableState::Neutral;
                    GfmClassificationTableEvent::Invalidate
                }
                None => {
                    self.state = GfmClassificationTableState::Neutral;
                    GfmClassificationTableEvent::NotGfm
                }
            },
        }
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

impl ClassificationFields {
    pub(super) fn record(
        &mut self,
        key: &str,
        value: &str,
        authority: Option<LaneAuthority>,
        gfm_display_row: bool,
    ) {
        if value.is_empty() {
            return;
        }
        match key {
            "lane type" => self.lane_type = true,
            "secondary surfaces" => self.secondary_surfaces = true,
            "owner decision" => {
                self.child_display_owner_decision = is_affirmative_child_owner_decision(value);
                self.child_owner_decision = authority.is_some_and(|authority| {
                    authority.authorizes_child_setup()
                        && if gfm_display_row {
                            is_affirmative_owner_decision_for(value, authority.owner())
                        } else {
                            !is_parent_owned_value(value)
                                && (is_affirmative_child_owned_value(value)
                                    || is_child_delegation_owner_decision(value))
                        }
                });
            }
            "atomic scope" => self.atomic_scope = true,
            "required skills" => self.required_skills = true,
            "required tools/evidence" | "required tools" | "required evidence" => {
                self.required_tools = true
            }
            "first allowed action" => self.first_allowed_action = true,
            key if Self::is_stop_blocker_key(key) => self.stop_blocker = true,
            _ => {}
        }
    }

    pub(super) fn records_key(key: &str) -> bool {
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
        ) || Self::is_stop_blocker_key(key)
    }

    pub(super) fn is_complete(&self) -> bool {
        self.has_required_fields() && self.child_owner_decision
    }

    pub(super) fn has_complete_child_display(&self) -> bool {
        self.has_required_fields() && self.child_display_owner_decision
    }

    fn has_required_fields(&self) -> bool {
        self.lane_type
            && self.secondary_surfaces
            && self.atomic_scope
            && self.required_skills
            && self.required_tools
            && self.first_allowed_action
            && self.stop_blocker
    }

    fn is_stop_blocker_key(key: &str) -> bool {
        matches!(key, "stop/blocker" | "stop blocker" | "blocker")
    }
}
