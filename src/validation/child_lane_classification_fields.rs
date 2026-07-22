use super::child_lane_classification_authority::LaneAuthority;
use super::child_lane_classification_schema::ClassificationTableSchema;
use super::child_lane_owner_decision::{
    is_affirmative_child_owner_decision, is_affirmative_owner_decision_for,
};
#[derive(Clone, Default)]
pub(super) struct ClassificationFields {
    next_field: usize,
    child_display_owner_decision: bool,
    child_owner_decision: bool,
}

impl ClassificationFields {
    pub(super) fn record(
        &mut self,
        key: &str,
        value: &str,
        authority: Option<LaneAuthority>,
    ) -> bool {
        if !ClassificationTableSchema::accepts(self.next_field, key, value) {
            return false;
        }
        if key.eq_ignore_ascii_case("owner decision") {
            let child_owner_decision = is_affirmative_child_owner_decision(value);
            self.child_display_owner_decision = child_owner_decision;
            self.child_owner_decision = authority.is_some_and(|authority| {
                authority.authorizes_child_setup()
                    && is_affirmative_owner_decision_for(value, authority.owner())
            });
        }
        self.next_field += 1;
        true
    }

    pub(super) fn records_key(key: &str) -> bool {
        ClassificationTableSchema::records_key(key)
    }

    pub(super) fn is_complete(&self) -> bool {
        self.has_required_fields() && self.child_owner_decision
    }

    pub(super) fn has_complete_child_display(&self) -> bool {
        self.has_required_fields() && self.child_display_owner_decision
    }

    pub(super) fn has_complete_shape(&self) -> bool {
        self.has_required_fields()
    }

    fn has_required_fields(&self) -> bool {
        self.next_field == ClassificationTableSchema::field_count()
    }
}
