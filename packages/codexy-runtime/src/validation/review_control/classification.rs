use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Result, bail};
use serde::Deserialize;

// v3 adds decisions for shared-contract risk and explicit strict requirements.
const SCHEMA: &str = "codexy.workflow-profile-classification.v3";
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
    "explicit_strict_requirement",
];
const LEGACY_COLLABORATION_TRIGGERS: [&str; 2] = ["durable_delegation", "multi_lane_ownership"];

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
    let known = MATERIAL_RISK_TRIGGERS
        .iter()
        .chain(LEGACY_COLLABORATION_TRIGGERS.iter())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let mut decisions = BTreeMap::new();
    for decision in input.strict_triggers {
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
    let non_strict = match (input.work_class.as_str(), input.low_risk_eligible) {
        ("low_risk", true) => "light",
        ("middle", false) => "standard",
        _ => bail!("workflow classification has no eligible non-strict review route"),
    };
    // Legacy collaboration flags remain parseable, but only material risk can promote review depth.
    Ok(
        if MATERIAL_RISK_TRIGGERS
            .iter()
            .any(|kind| decisions.get(*kind).is_some_and(|applies| *applies))
        {
            "strict".into()
        } else {
            non_strict.into()
        },
    )
}
