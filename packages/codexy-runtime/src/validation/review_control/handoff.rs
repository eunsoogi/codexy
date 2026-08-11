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
    let Ok(profiles) = policy::load(plugin_root) else {
        return vec!["review-profile policy is unavailable".into()];
    };
    let Some(selected) = pr_state.get("reviewProfile").and_then(Value::as_str) else {
        return vec!["profile-routed review selection must be typed and closed".into()];
    };
    let Some(profile) = profiles.get(selected) else {
        return vec!["profile-routed review selection names an unknown profile".into()];
    };
    let Some(raw) = pr_state.get("reviewEvidence") else {
        return profile
            .reviewer
            .is_none()
            .then(Vec::new)
            .unwrap_or_else(|| vec!["profile-routed review evidence must be present".into()]);
    };
    if profile.reviewer.is_none() {
        return vec!["light review selection must not attach reviewer evidence".into()];
    }
    let Ok(evidence) = serde_json::from_value::<Evidence>(raw.clone()) else {
        return vec!["profile-routed review evidence must be typed and closed".into()];
    };
    if evidence.schema != "codexy.review-readiness.v1"
        || evidence.profile != selected
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
