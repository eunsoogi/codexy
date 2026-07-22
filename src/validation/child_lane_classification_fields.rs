use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};

pub(super) fn classification_table_row(line: &str) -> Option<(&str, &str)> {
    let row = line.strip_prefix('|')?.strip_suffix('|')?;
    let (key, value) = row.split_once('|')?;
    (!value.contains('|')).then_some((key.trim(), value.trim()))
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
                self.child_owner_decision =
                    is_child_completion_owner(value) || is_current_thread_owner(value);
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

fn is_current_thread_owner(value: &str) -> bool {
    let Some(rationale) = value.strip_prefix("current-thread-owned") else {
        return false;
    };
    let rationale = rationale.trim();
    !rationale.is_empty()
        && !value.contains("not current-thread-owned")
        && !rationale.starts_with("or ")
        && !["parent-owned", "unknown", "ambiguous"]
            .iter()
            .any(|marker| rationale.contains(marker))
}

fn is_child_completion_owner(value: &str) -> bool {
    !is_parent_owned_value(value) && is_child_delegation_owner_decision(value)
}
