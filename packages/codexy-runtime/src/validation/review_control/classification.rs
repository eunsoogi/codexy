use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Result, bail};
use serde::Deserialize;

const SCHEMA: &str = "codexy.workflow-profile-classification.v2";
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

pub(super) fn check(_plugin_root: &Path) -> Result<()> {
    Ok(())
}

pub(super) fn select(_plugin_root: &Path, input: Input) -> Result<String> {
    if input.schema != SCHEMA {
        bail!("workflow classification has an unsupported schema");
    }
    let mut decisions = BTreeMap::new();
    for decision in input.strict_triggers {
        if decisions.insert(decision.kind, decision.applies).is_some() {
            bail!("workflow classification duplicates a strict trigger decision");
        }
    }
    let expected = STRICT_TRIGGERS
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
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
