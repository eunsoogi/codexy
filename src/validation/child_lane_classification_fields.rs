use super::child_lane_classification_authority::LaneAuthority;
use super::child_lane_owner_decision::{
    is_affirmative_child_owned_value, is_affirmative_child_owner_decision,
    is_affirmative_owner_decision_for, is_child_delegation_owner_decision, is_parent_owned_value,
};
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
                let child_owner_decision = if gfm_display_row {
                    is_affirmative_child_owner_decision(value)
                } else {
                    !is_parent_owned_value(value)
                        && (is_affirmative_child_owned_value(value)
                            || is_child_delegation_owner_decision(value))
                };
                self.child_display_owner_decision = child_owner_decision;
                self.child_owner_decision = authority.is_some_and(|authority| {
                    authority.authorizes_child_setup()
                        && if gfm_display_row {
                            is_affirmative_owner_decision_for(value, authority.owner())
                        } else {
                            child_owner_decision
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
