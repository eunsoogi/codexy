use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Result, bail};
use serde::Deserialize;

const PATH: &str = "skills/orchestration/references/workflow-review-classification.json";
const SCHEMA: &str = "codexy.workflow-profile-classification.v2";
const POLICY_SCHEMA: &str = "codexy.workflow-review-classification-policy.v1";
const STRICT_TRIGGERS: [&str; 11] = [
    "destructive",
    "security",
    "permission",
    "secret",
    "release",
    "high_consequence_external_state",
    "high_risk_guardrail",
    "merge_sensitive",
    "durable_delegation",
    "multi_lane_ownership",
    "explicit_audit_evidence",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: String,
    work_classes: Vec<String>,
    strict_triggers: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    schema: String,
    work_class: String,
    low_risk_eligible: bool,
    strict_triggers: Vec<TriggerDecision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TriggerDecision {
    kind: String,
    applies: bool,
}

pub(super) fn check(plugin_root: &Path) -> Result<()> {
    load(plugin_root).map(|_| ())
}

pub(super) fn select(plugin_root: &Path, input: Input) -> Result<String> {
    let policy = load(plugin_root)?;
    if input.schema != SCHEMA {
        bail!("workflow classification has an unsupported schema");
    }
    let mut decisions = BTreeMap::new();
    for decision in input.strict_triggers {
        if decisions.insert(decision.kind, decision.applies).is_some() {
            bail!("workflow classification duplicates a strict trigger decision");
        }
    }
    let expected = policy.strict_triggers.into_iter().collect::<BTreeSet<_>>();
    if decisions.keys().cloned().collect::<BTreeSet<_>>() != expected {
        bail!("workflow classification must decide every closed strict trigger");
    }
    let non_strict = match (input.work_class.as_str(), input.low_risk_eligible) {
        ("low_risk", true) => "light",
        ("middle", false) => "standard",
        _ => bail!("workflow classification has no eligible non-strict review route"),
    };
    Ok(if decisions.values().any(|applies| *applies) {
        "strict".into()
    } else {
        non_strict.into()
    })
}

fn load(plugin_root: &Path) -> Result<Policy> {
    let text = fs::read_to_string(plugin_root.join(PATH))?;
    let policy: Policy = serde_json::from_str(&text)?;
    let expected = STRICT_TRIGGERS
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if policy.schema != POLICY_SCHEMA
        || policy.work_classes != ["low_risk", "middle"]
        || policy.strict_triggers != expected
    {
        bail!("workflow review classification policy must retain the closed typed contract");
    }
    Ok(policy)
}
