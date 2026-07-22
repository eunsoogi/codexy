use super::child_lane_classification_authority::LaneAuthority;
use super::child_lane_classification_schema::ClassificationTableSchema;
use super::child_lane_owner_decision::{
    OwnerSelection, is_affirmative_child_owner_decision, is_affirmative_owner_decision_for,
    is_parent_owned_value,
};
#[derive(Clone, Default)]
pub(super) struct ClassificationFields {
    next_field: usize,
    child_display_owner_decision: bool,
    child_owner_decision: bool,
    authority_owner_decision: bool,
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
            self.authority_owner_decision = authority.is_some_and(|authority| {
                is_affirmative_owner_decision_for(value, authority.owner())
                    || (authority.owner() == OwnerSelection::ParentOwned
                        && is_parent_owned_value(value))
            });
            self.child_owner_decision = authority
                .is_some_and(|authority| authority.authorizes_child_setup())
                && self.authority_owner_decision;
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

    pub(super) fn has_complete_authority_record(&self) -> bool {
        self.has_required_fields() && self.authority_owner_decision
    }

    fn has_required_fields(&self) -> bool {
        self.next_field == ClassificationTableSchema::field_count()
    }
}
