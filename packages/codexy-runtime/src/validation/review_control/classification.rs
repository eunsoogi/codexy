use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Result, bail};
use serde::Deserialize;

// Complete risk decisions remain independent of optional reviewer selection.
const SCHEMA: &str = "codexy.risk-assessment.v1";
const MATERIAL_RISK_TRIGGERS: [&str; 11] = [
    "destructive",
    "security",
    "permission",
    "secret",
    "release",
    "high_consequence_external_state",
    "high_risk_guardrail",
    "merge_sensitive",
    "materially_shared_executable_contract",
    "explicit_audit_evidence",
    "explicit_review_requirement",
];
const LEGACY_COLLABORATION_TRIGGERS: [&str; 2] = ["durable_delegation", "multi_lane_ownership"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    schema: String,
    risk_decisions: Vec<TriggerDecision>,
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

pub(super) fn select(_plugin_root: &Path, input: Input) -> Result<bool> {
    if input.schema != SCHEMA {
        bail!("workflow classification has an unsupported schema");
    }
    let known = MATERIAL_RISK_TRIGGERS
        .iter()
        .chain(LEGACY_COLLABORATION_TRIGGERS.iter())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let mut decisions = BTreeMap::new();
    for decision in input.risk_decisions {
        if !known.contains(&decision.kind) {
            bail!("workflow classification has an unknown trigger decision");
        }
        if decisions.insert(decision.kind, decision.applies).is_some() {
            bail!("workflow classification duplicates a trigger decision");
        }
    }
    let required = MATERIAL_RISK_TRIGGERS
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    if !required.is_subset(&decisions.keys().cloned().collect()) {
        bail!("workflow classification must decide every material-risk trigger");
    }
    // Only an explicit review obligation requires review; risk alone selects no tier.
    Ok(decisions
        .get("explicit_review_requirement")
        .copied()
        .unwrap_or(false))
}
