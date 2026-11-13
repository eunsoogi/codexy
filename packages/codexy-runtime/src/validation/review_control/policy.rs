use std::{collections::BTreeMap, path::Path};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::classification;

const REQUEST_SCHEMA: &str = "codexy.review-selection-request.v2";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Profile {
    pub(super) reviewer: Option<Reviewer>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Reviewer {
    pub(super) name: String,
    pub(super) model: String,
    pub(super) reasoning_effort: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    risk_assessment: classification::Input,
    review_requested: bool,
}

pub(super) fn load(plugin_root: &Path) -> Result<BTreeMap<String, Profile>> {
    let _ = plugin_root;
    Ok(profiles())
}

pub(super) fn resolve(plugin_root: &Path, text: &str) -> Result<Value> {
    let request: Request = serde_json::from_str(text)?;
    if request.schema != REQUEST_SCHEMA {
        bail!("review-profile request has an unsupported schema");
    }
    // Preserve complete risk-assessment validation; review selection is independent.
    let required = classification::select(plugin_root, request.risk_assessment)?;
    if required && !request.review_requested {
        bail!("an explicit review requirement must not be skipped");
    }
    Ok(json!({
        "review_requested": request.review_requested,
        "reviewer": request.review_requested.then(sentinel),
        "stage": "final",
    }))
}

pub(super) fn sentinel() -> Reviewer {
    Reviewer {
        name: "codexy-sentinel".into(),
        model: "gpt-6.1-sol".into(),
        reasoning_effort: "high".into(),
    }
}

// Read-only compatibility for historical v1 controls, not new-work selection.
// Never reinterpret an old review as evidence from the new reviewer identity.
fn profiles() -> BTreeMap<String, Profile> {
    BTreeMap::from([
        ("light".into(), Profile { reviewer: None }),
        (
            "standard".into(),
            Profile {
                reviewer: Some(Reviewer {
                    name: "codexy-inspector".into(),
                    model: "gpt-6.1-sol".into(),
                    reasoning_effort: "medium".into(),
                }),
            },
        ),
        (
            "strict".into(),
            Profile {
                reviewer: Some(Reviewer {
                    name: "codexy-sentinel".into(),
                    model: "gpt-6-astra".into(),
                    reasoning_effort: "xhigh".into(),
                }),
            },
        ),
    ])
}
