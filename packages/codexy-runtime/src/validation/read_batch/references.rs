use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadReference {
    pub kind: String,
    pub locator: String,
    pub read_only: bool,
    pub independent: bool,
}

impl ReadReference {
    #[must_use]
    pub fn is_eligible(&self) -> bool {
        !self.kind.trim().is_empty()
            && !self.locator.trim().is_empty()
            && self.read_only
            && self.independent
    }
}

#[must_use]
pub fn issue_reference(number: u64) -> ReadReference {
    ReadReference {
        kind: "issue".to_owned(),
        locator: format!("#{number}"),
        read_only: true,
        independent: true,
    }
}
