use super::child_lane_classification_authority::LaneAuthority;
use super::child_lane_classification_fields::ClassificationFields;
use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};

#[derive(Default)]
pub(super) struct ColonClassificationBlock {
    state: ColonClassificationBlockState,
}

#[derive(Default)]
enum ColonClassificationBlockState {
    #[default]
    Active,
    Complete,
    Terminated,
}

impl ColonClassificationBlock {
    pub(super) fn replace_with_gfm(&mut self, fields: &mut ClassificationFields) {
        *fields = ClassificationFields::default();
        self.state = ColonClassificationBlockState::Active;
    }

    pub(super) fn invalidate(&mut self, fields: &mut ClassificationFields) {
        *fields = ClassificationFields::default();
        self.state = ColonClassificationBlockState::Terminated;
    }

    pub(super) fn record_gfm(
        &mut self,
        fields: &mut ClassificationFields,
        key: &str,
        value: &str,
        authority: Option<LaneAuthority>,
    ) {
        self.record(fields, key, value, authority, true);
    }

    pub(super) fn consume_colon(
        &mut self,
        fields: &mut ClassificationFields,
        line: &str,
        authority: Option<LaneAuthority>,
    ) {
        if !matches!(self.state, ColonClassificationBlockState::Active) {
            return;
        }
        let Some((key, value)) = line.split_once(':') else {
            if !line.is_empty() {
                self.state = ColonClassificationBlockState::Terminated;
            }
            return;
        };
        let key = metadata_key(key);
        if ClassificationFields::records_key(key) {
            self.record(fields, key, trimmed_value(value), authority, false);
        } else {
            self.state = ColonClassificationBlockState::Terminated;
        }
    }

    fn record(
        &mut self,
        fields: &mut ClassificationFields,
        key: &str,
        value: &str,
        authority: Option<LaneAuthority>,
        gfm_display_row: bool,
    ) {
        if !matches!(self.state, ColonClassificationBlockState::Active) {
            return;
        }
        if !fields.record(key, value, authority, gfm_display_row) {
            self.invalidate(fields);
        } else if fields.has_complete_shape() {
            self.state = ColonClassificationBlockState::Complete;
        }
    }
}
