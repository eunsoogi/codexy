use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};

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

#[derive(Default)]
pub(super) struct ClassificationFields {
    pub(super) table_header: bool,
    pub(super) table_separator: bool,
    lane_type: bool,
    secondary_surfaces: bool,
    atomic_scope: bool,
    required_skills: bool,
    required_tools: bool,
    first_allowed_action: bool,
    stop_blocker: bool,
    child_owner_decision: bool,
}

impl ClassificationFields {
    pub(super) fn record(&mut self, key: &str, value: &str) {
        if value.is_empty() {
            return;
        }
        match key {
            "lane type" => self.lane_type = true,
            "secondary surfaces" => self.secondary_surfaces = true,
            "owner decision" => {
                self.child_owner_decision = is_child_completion_owner(value);
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
        self.lane_type
            && self.secondary_surfaces
            && self.atomic_scope
            && self.required_skills
            && self.required_tools
            && self.first_allowed_action
            && self.stop_blocker
            && self.child_owner_decision
    }

    fn is_stop_blocker_key(key: &str) -> bool {
        matches!(key, "stop/blocker" | "stop blocker" | "blocker")
    }
}

fn is_child_completion_owner(value: &str) -> bool {
    !is_parent_owned_value(value) && is_child_delegation_owner_decision(value)
}
