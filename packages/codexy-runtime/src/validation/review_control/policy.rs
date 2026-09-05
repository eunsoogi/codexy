use std::{collections::BTreeMap, path::Path};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::classification;

const REQUEST_SCHEMA: &str = "codexy.review-profile-request.v1";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Profile {
    pub(super) reviewer: Option<Reviewer>,
    pub(super) full_review_limit: u8,
    pub(super) delta_recheck_limit: u8,
    pub(super) terminal_review_limit: u8,
    pub(super) post_cap_re_review_limit: u8,
    pub(super) max_blocking_findings: u8,
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
    classification: classification::Input,
    #[serde(default)]
    prior_profile: Option<String>,
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
    let profiles = profiles();
    let selected = classification::select(plugin_root, request.classification)?;
    let profile = profiles
        .get(&selected)
        .ok_or_else(|| anyhow::anyhow!("review-profile request names an unknown profile"))?;
    let mut route = json!({
        "profile": selected,
        "reviewer": profile.reviewer,
        "full_review_limit": profile.full_review_limit,
        "delta_recheck_limit": profile.delta_recheck_limit,
        "terminal_review_limit": profile.terminal_review_limit,
        "post_cap_re_review_limit": profile.post_cap_re_review_limit,
    });
    if let Some(prior) = request.prior_profile {
        if rank(&prior) >= rank(route["profile"].as_str().unwrap_or_default()) {
            bail!("review-profile escalation must select a strictly higher profile");
        }
        route["discarded_lower_profile"] = Value::String(prior);
    }
    Ok(route)
}

fn rank(profile: &str) -> u8 {
    match profile {
        "light" => 0,
        "standard" => 1,
        "strict" => 2,
        _ => u8::MAX,
    }
}

fn profiles() -> BTreeMap<String, Profile> {
    BTreeMap::from([
        (
            "light".to_owned(),
            Profile {
                reviewer: None,
                full_review_limit: 0,
                delta_recheck_limit: 0,
                terminal_review_limit: 0,
                post_cap_re_review_limit: 0,
                max_blocking_findings: 0,
            },
        ),
        (
            "standard".to_owned(),
            Profile {
                reviewer: Some(Reviewer {
                    name: "codexy-inspector".into(),
                    model: "gpt-5.6-sol".into(),
                    reasoning_effort: "medium".into(),
                }),
                full_review_limit: 1,
                delta_recheck_limit: 1,
                terminal_review_limit: 3,
                post_cap_re_review_limit: 1,
                max_blocking_findings: 3,
            },
        ),
        (
            "strict".to_owned(),
            Profile {
                reviewer: Some(Reviewer {
                    name: "codexy-sentinel".into(),
                    model: "gpt-6-astra".into(),
                    reasoning_effort: "xhigh".into(),
                }),
                full_review_limit: 1,
                delta_recheck_limit: 1,
                terminal_review_limit: 3,
                post_cap_re_review_limit: 1,
                max_blocking_findings: u8::MAX,
            },
        ),
    ])
}
