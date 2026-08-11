use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::policy::{self, Reviewer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema: String,
    head_oid: String,
    profile: String,
    reviewer: Option<Reviewer>,
    state: String,
}

pub(super) fn check(plugin_root: &Path, pr_state: &Value) -> Vec<String> {
    let Some(raw) = pr_state.get("reviewEvidence") else {
        return vec!["profile-routed review evidence must be present".into()];
    };
    let Ok(evidence) = serde_json::from_value::<Evidence>(raw.clone()) else {
        return vec!["profile-routed review evidence must be typed and closed".into()];
    };
    let Ok(profiles) = policy::load(plugin_root) else {
        return vec!["review-profile policy is unavailable".into()];
    };
    let Some(profile) = profiles.get(&evidence.profile) else {
        return vec!["profile-routed review evidence names an unknown profile".into()];
    };
    if evidence.schema != "codexy.review-readiness.v1"
        || pr_state.get("headRefOid").and_then(Value::as_str) != Some(&evidence.head_oid)
        || evidence.reviewer != profile.reviewer
        || evidence.state != "passed"
    {
        return vec![
            "profile-routed review evidence must bind the selected reviewer and current head PASS"
                .into(),
        ];
    }
    Vec::new()
}
